
use core::cell::RefCell;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{Config, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use esp_println::println;
use esp_wifi::{EspWifiInitFor, initialize};
use esp_wifi::wifi::{AuthMethod, ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use hal::{embassy, Rng};
use hal::clock::Clocks;
use hal::peripherals::{SYSTIMER, WIFI};
use hal::system::RadioClockControl;
use static_cell::{make_static, StaticCell};

#[derive(Eq, PartialEq)]
enum WifiNetState {
    WifiConnecting,
    WifiConnected,
    WifiDisconnected,
    WifiStopped,
}
#[derive(Debug)]
pub enum WifiNetError {
    WaitConnecting,
    TimeOut,
    Infallible,
    Using,
}




const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const HOW_LONG_SECS_CLOSE:u64 = 30;//20秒未使用wifi 断开


pub static STOP_WIFI_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static RECONNECT_WIFI_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static LAST_USE_TIME_SECS:Mutex<CriticalSectionRawMutex,RefCell<u64>>  =  Mutex::new(RefCell::new(0));
pub static WIFI_STATE:Mutex<CriticalSectionRawMutex,RefCell<WifiNetState>>  =  Mutex::new(RefCell::new(WifiNetState::WifiStopped));
pub static mut STACK_MUT: Option<&'static Stack<WifiDevice<'static, WifiStaDevice>>>  =  None;

pub static HAL_RNG:Mutex<CriticalSectionRawMutex,Option<Rng>>  =  Mutex::new(None);
struct WifiInstance{
}

pub async fn connect_wifi(spawner: &Spawner,
                          systimer: SYSTIMER,
                          rng: Rng,
                          wifi: WIFI,
                          radio_clock_control: RadioClockControl,
                          clocks: &Clocks<'_> )
    -> Result<&'static Stack<WifiDevice<'static, WifiStaDevice>>, WifiNetError> {

    (*HAL_RNG.lock().await).replace(rng);

    let timer = hal::systimer::SystemTimer::new(systimer).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        rng,
        radio_clock_control,
        &clocks,
    )
        .unwrap();

    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed
    ));


    LAST_USE_TIME_SECS.lock().await.replace(Instant::now().as_secs());

    spawner.spawn(connection_wifi(controller)).ok();
    spawner.spawn(net_task(stack)).ok();
    spawner.spawn(do_stop()).ok();

    loop {
        println!("Waiting is_link_up...");
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(1000)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
  /*  STACK_MUT.lock().await.replace(Some(stack));*/
    unsafe {
        STACK_MUT = Some(stack);
    }
    Ok(stack)
}


#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}

#[embassy_executor::task]
async fn connection_wifi(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                WIFI_STATE.lock().await.replace(WifiNetState::WifiConnected);
                let disconnect =  controller.wait_for_event(WifiEvent::StaDisconnected);
                let closeconnect = STOP_WIFI_SIGNAL.wait();

                match select(disconnect,closeconnect).await {
                    Either::First(_) => {
                        WIFI_STATE.lock().await.replace(WifiNetState::WifiDisconnected);
                        Timer::after(Duration::from_millis(1000)).await
                    }
                    Either::Second(_) => {
                        STOP_WIFI_SIGNAL.reset();
                        controller.stop().await.expect("wifi stop error");
                        println!("wifi close...");
                        WIFI_STATE.lock().await.replace(WifiNetState::WifiStopped);
                        RECONNECT_WIFI_SIGNAL.wait().await;
                        RECONNECT_WIFI_SIGNAL.reset();
                        println!("restart connect...");
                        WIFI_STATE.lock().await.replace(WifiNetState::WifiDisconnected);

                    }
                }

            }
            _ => { WIFI_STATE.lock().await.replace(WifiNetState::WifiDisconnected);}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method: AuthMethod::None,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        WIFI_STATE.lock().await.replace(WifiNetState::WifiConnecting);
        match controller.connect().await {
            Ok(_) =>{
                println!("Wifi connected!");
                WIFI_STATE.lock().await.replace(WifiNetState::WifiConnected);

            },
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

async fn refresh_last_time(){
    LAST_USE_TIME_SECS.lock().await.replace(Instant::now().as_secs());
}


//每次请求都要获取stack 并修改 WIFI_LOCK 为true,用完后 改回，同时多个任务容易崩
const TIME_OUT_SECS: u64 = 10;
static WIFI_LOCK:Mutex<CriticalSectionRawMutex,bool> = Mutex::new(false);
pub async fn use_wifi() ->Result<&'static Stack<WifiDevice<'static, WifiStaDevice>>, WifiNetError>{
    let secs = Instant::now().as_secs();

    if *WIFI_STATE.lock().await.get_mut() != WifiNetState::WifiConnected {
        println!("need wait");
    }
    if *WIFI_STATE.lock().await.get_mut() == WifiNetState::WifiStopped {
        println!("send reconnect signal...");
        RECONNECT_WIFI_SIGNAL.signal(());
    }



    loop {
        refresh_last_time().await;
        println!("use_wifi Waiting is_link_up...");
        unsafe {
            if let Some(v) = STACK_MUT {
                if v.is_link_up() {

                    loop {
                        if !*WIFI_LOCK.lock().await  {
                            break;
                        }
                        if Instant::now().as_secs() - secs > TIME_OUT_SECS  {
                            return Err(WifiNetError::TimeOut);
                        }
                        Timer::after(Duration::from_millis(50)).await;
                    }
                    *WIFI_LOCK.lock().await = true;
                    v.wait_config_up().await;
                    return Ok(v);

                }else if Instant::now().as_secs() - secs > TIME_OUT_SECS {
                    return Err(WifiNetError::TimeOut);
                }
            }else{
                return Err(WifiNetError::Infallible);
            }
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

pub async fn finish_wifi(){

    *WIFI_LOCK.lock().await   = false;
    println!("finish_wifi");
}


#[embassy_executor::task]
async fn do_stop(){
    loop {

        if  *WIFI_STATE.lock().await.get_mut() == WifiNetState::WifiConnected {
            if Instant::now().as_secs() - *LAST_USE_TIME_SECS.lock().await.get_mut() > HOW_LONG_SECS_CLOSE {
                println!("do_stop_wifi");
                STOP_WIFI_SIGNAL.signal(());
                finish_wifi().await;
            }
        }
        Timer::after(Duration::from_millis(3000)).await
    }
}

