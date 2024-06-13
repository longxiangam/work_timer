use alloc::string::ToString;
use core::cell::RefCell;
use core::net::Ipv4Addr;
use core::ops::DerefMut;
use core::str::FromStr;
use dhcparse::dhcpv4::{Addr, DhcpOption, Encode, Encoder, Message};
use dhcparse::v4_options;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::{Config, IpAddress, IpEndpoint, IpListenEndpoint, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
use embassy_net::tcp::{AcceptError, TcpSocket};
use embassy_net::udp::UdpSocket;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use esp_println::{print, println};
use esp_wifi::{EspWifiInitFor, initialize};
use esp_wifi::wifi::{AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration, WifiApDevice, WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::wifi::ipv4::{ RouterConfiguration, SocketAddrV4};
use hal::{embassy, Rng};
use hal::clock::Clocks;
use hal::peripherals::{SYSTIMER, WIFI};
use hal::system::RadioClockControl;
use heapless::{String, Vec};
use static_cell::{make_static, StaticCell};

#[derive(Eq, PartialEq,Copy, Clone)]
pub enum WifiModel{
    AP,
    STA,
}
#[derive(Eq, PartialEq,Copy, Clone)]
pub enum WifiNetState {
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

pub static mut IP_ADDRESS:String<20> = String::new();
pub static STOP_WIFI_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static RECONNECT_WIFI_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static STOP_WEB_SERVICE: Signal<CriticalSectionRawMutex,()> = Signal::new();
pub static LAST_USE_TIME_SECS:Mutex<CriticalSectionRawMutex,RefCell<u64>>  =  Mutex::new(RefCell::new(0));
pub static WIFI_STATE:Mutex<CriticalSectionRawMutex,RefCell<WifiNetState>>  =  Mutex::new(RefCell::new(WifiNetState::WifiStopped));
pub static mut STACK_MUT: Option<&'static Stack<WifiDevice<'static, WifiStaDevice>>>  =  None;
pub static mut AP_STACK_MUT: Option<&'static Stack<WifiDevice<'static, WifiApDevice>>>  =  None;

pub static HAL_RNG:Mutex<CriticalSectionRawMutex,Option<Rng>>  =  Mutex::new(None);
pub static WIFI_MODEL:Mutex<CriticalSectionRawMutex,Option<WifiModel>> = Mutex::new(None);
pub async fn connect_wifi(spawner: &Spawner,
                          systimer: SYSTIMER,
                          rng: Rng,
                          wifi: WIFI,
                          radio_clock_control: RadioClockControl,
                          clocks: &Clocks<'_> )
    -> Result<&'static Stack<WifiDevice<'static, WifiStaDevice>>, WifiNetError> {

    HAL_RNG.lock().await.replace(rng);

    let timer = hal::systimer::SystemTimer::new(systimer).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        rng,
        radio_clock_control,
        &clocks,
    )
        .unwrap();

    #[cfg(not(feature = "wifi_ap"))]
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    #[cfg(feature = "wifi_ap")]
        let (wifi_ap_interface,wifi_interface,mut controller) =
            esp_wifi::wifi::new_ap_sta(&init, wifi).unwrap();

    #[cfg(feature = "wifi_ap")]
    {
        let seed = 1234;
        let ap_config = Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
            gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
            dns_servers: Default::default(),
        });
        let ap_stack: &Stack<WifiDevice<'static, WifiApDevice>> = &*make_static!(
            Stack::new(
                wifi_ap_interface,
                ap_config,
                make_static!( StackResources::<3>::new()),
                seed
            )
        );

       
        spawner.spawn(ap_task(&ap_stack)).ok();
        unsafe {
            AP_STACK_MUT = Some(ap_stack);
        }
        let client_config = ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            password: PASSWORD.try_into().unwrap(),
            auth_method: AuthMethod::None,
            ..Default::default()
        };
        let ap_config =  AccessPointConfiguration {
            ssid: "esp-wifi".try_into().unwrap(),
            ..Default::default()
        };
        let config = make_static!(Configuration::Mixed(client_config,ap_config));
        controller.set_configuration(config);

        spawner.spawn(dhcp_service()).ok();

    }

    let config = Config::dhcpv4(Default::default());

    let seed = 1234;

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
            unsafe {
                IP_ADDRESS =  config.address.address().to_string().parse().unwrap();
            }
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    unsafe {
        STACK_MUT = Some(stack);
    }
    Ok(stack)
}

#[embassy_executor::task]
async fn ap_task(stack: &'static Stack<WifiDevice<'static, WifiApDevice>>) {
    stack.run().await
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

pub async fn refresh_last_time(){
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



/// ap 模式 配网
pub async fn start_wifi_ap(spawner: &Spawner,
                           systimer: SYSTIMER,
                           rng: Rng,
                           wifi: WIFI,
                           radio_clock_control: RadioClockControl,
                           clocks: &Clocks<'_> )
                           -> Result<&'static Stack<WifiDevice<'static, WifiApDevice>>, WifiNetError> {

    HAL_RNG.lock().await.replace(rng);

    let timer = hal::systimer::SystemTimer::new(systimer).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        rng,
        radio_clock_control,
        &clocks,
    )
        .unwrap();

    let (wifi_ap_interface, mut controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiApDevice).unwrap();

    let seed = 1234;
    let ap_config = Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
        gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
        dns_servers: Default::default(),
    });
    let ap_stack: &Stack<WifiDevice<'static, WifiApDevice>> = &*make_static!(
            Stack::new(
                wifi_ap_interface,
                ap_config,
                make_static!( StackResources::<3>::new()),
                seed
            )
        );

    spawner.spawn(ap_task(&ap_stack)).ok();
    spawner.spawn(dhcp_service()).ok();
    spawner.spawn(connection_wifi_ap(controller)).ok();




    loop {
        println!("Waiting is_link_up...");
        if ap_stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(1000)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = ap_stack.config_v4() {
            println!("Got IP: {}", config.address);
            unsafe {
                IP_ADDRESS =  config.address.address().to_string().parse().unwrap();
            }
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    unsafe {
        AP_STACK_MUT = Some(ap_stack);
    }

    Ok(ap_stack)
}

#[embassy_executor::task]
async fn connection_wifi_ap(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::ApStarted => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::AccessPoint(AccessPointConfiguration {
                ssid: "esp-wifi".try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
    }
}

#[embassy_executor::task]
async fn dhcp_service(){
    const RX_BUFFER_SIZE: usize = 512; // 接收缓冲区大小
    const TX_BUFFER_SIZE: usize = 512; // 发送缓冲区大小
    const PACKET_META_SIZE: usize = 10; // 元数据大小


    let mut ip_pool:heapless::Vec<Ipv4Addr,10> = heapless::Vec::new();
    ip_pool.push( Ipv4Addr::new(192, 168, 2, 2));
    ip_pool.push( Ipv4Addr::new(192, 168, 2, 3));
    ip_pool.push( Ipv4Addr::new(192, 168, 2, 4));
    ip_pool.push( Ipv4Addr::new(192, 168, 2, 5));

    loop {

        unsafe {
            if let Some(ap_stack) = AP_STACK_MUT {
                loop {
                    if ap_stack.is_link_up() {
                        break;
                    }
                    Timer::after(Duration::from_millis(500)).await;
                }

                let mut rx_meta = [embassy_net::udp::PacketMetadata::EMPTY; PACKET_META_SIZE];
                let mut rx_buffer = [0u8; RX_BUFFER_SIZE];
                let mut tx_meta = [embassy_net::udp::PacketMetadata::EMPTY; PACKET_META_SIZE];
                let mut tx_buffer = [0u8; TX_BUFFER_SIZE];
                let mut udp_socket = UdpSocket::new(ap_stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
                udp_socket.bind(67);

                // 无限循环处理消息
                loop {
                    let mut buf = [0u8; 512];
                    println!("等待请求") ;
                    match udp_socket.recv_from(&mut buf).await {
                        Ok((n, src)) => {
                            println!("Received {} bytes from {}", n, src);
                            println!("Received:{:?} ", buf );

                            let mut msg = Message::new(buf).unwrap();
                            println!("msg op_type:{:?}",msg.op().unwrap()) ;
                            let options =  v4_options!(msg; MessageType required, ServerIdentifier, RequestedIpAddress);
                            match options {
                                Ok((msg_type,si,ria)) => {
                                    println!("msg type:{:?}",msg_type) ;
                                    if msg_type == dhcparse::dhcpv4::MessageType::DISCOVER {
                                        send_dhcp_offer(&udp_socket, src ,&msg).await;
                                    }
                                    else if msg_type ==  dhcparse::dhcpv4::MessageType::REQUEST {
                                        let ip_addr = Ipv4Addr::new(192, 168, 2, 2);
                                        send_dhcp_ack(&udp_socket, src, &msg).await;
                                    }
                                }
                                Err(_) => {}
                            }

                            //udp_socket.send_to(&buf[..n], src).await;
                        }
                        Err(e) => {
                            println!("Failed to receive UDP packet: {:?}", e);
                        }
                    }

                    Timer::after(Duration::from_secs(1)).await;
                }
            }
        }

        Timer::after(Duration::from_millis(500)).await
    }
}

async fn send_dhcp_offer(udp_socket: &UdpSocket<'_>, src_addr: IpEndpoint, receive_msg: &Message<[u8; 512]>) {
    println!("send_dhcp_offer") ;
    // 构造并发送 DHCP Offer 消息
    let router_ip:&Addr = (&[192u8,168,2,1][..]).try_into().unwrap();
    let submask:&Addr = (&[255u8,255,255,0][..]).try_into().unwrap();

    let mut offer_message = [0u8; 512];

    offer_message[2] = 6;

    let mut msg = Encoder
        .append_options([DhcpOption::MessageType(dhcparse::dhcpv4::MessageType::OFFER)])
        .append_options([DhcpOption::Router(&[*router_ip])])
        .append_options([DhcpOption::SubnetMask(submask)])
        .append_options([DhcpOption::AddressLeaseTime(3600)])
        .append_options([DhcpOption::ServerIdentifier(router_ip)])
        .append_options([DhcpOption::DomainNameServer(&[*router_ip])])
        .encode(&Message::default(), &mut offer_message).unwrap();
    msg.set_op(dhcparse::dhcpv4::OpCode::BootReply);
    msg.set_xid(receive_msg.xid());
    msg.set_chaddr(receive_msg.chaddr().unwrap());


    let temp :[u8;4] = [192,168,2,1];
    let si_addr:&Addr = (&temp[..]).try_into().unwrap();
    *msg.siaddr_mut() = *si_addr;


    let temp :[u8;4] = [192,168,2,2];
    let yi_addr:&Addr = (&temp[..]).try_into().unwrap();
    *msg.yiaddr_mut() = *yi_addr;

    offer_message[1] = 1;
    println!("{:?}",&offer_message);


    let broadcast = ( Ipv4Address::BROADCAST,68);
    udp_socket.send_to(&offer_message, broadcast).await;
}

async fn send_dhcp_ack(udp_socket: & UdpSocket<'_>, src_addr: IpEndpoint, receive_msg: &Message<[u8; 512]>) {
    println!("send_dhcp_ack") ;
    // 构造并发送 DHCP Acknowledge 消息
    let router_ip:&Addr = (&[192u8,168,2,1][..]).try_into().unwrap();
    let submask:&Addr = (&[255u8,255,255,0][..]).try_into().unwrap();
    let mut offer_message = [0u8; 512];
    offer_message[1] = 1;
    offer_message[2] = 6;

    let mut msg = Encoder
        .append_options([DhcpOption::MessageType(dhcparse::dhcpv4::MessageType::ACK)])
        .append_options([DhcpOption::Router(&[*router_ip])])
        .append_options([DhcpOption::SubnetMask(submask)])
        .append_options([DhcpOption::AddressLeaseTime(3600)])
        .append_options([DhcpOption::ServerIdentifier(router_ip)])
        .append_options([DhcpOption::DomainNameServer(&[*router_ip])])
        .encode(&Message::default(), &mut offer_message).unwrap();
    msg.set_op(dhcparse::dhcpv4::OpCode::BootReply);
    msg.set_xid(receive_msg.xid());
    msg.set_chaddr(receive_msg.chaddr().unwrap());

    let temp :[u8;4] = [192,168,2,1];
    let si_addr:&Addr = (&temp[..]).try_into().unwrap();
    *msg.siaddr_mut() = *si_addr;



    let temp :[u8;4] = [192,168,2,2];
    let yi_addr:&Addr = (&temp[..]).try_into().unwrap();
    *msg.yiaddr_mut() = *yi_addr;

    offer_message[1] = 1;
    println!("{:?}",&offer_message);

    let broadcast = ( Ipv4Address::BROADCAST,68);
    udp_socket.send_to(&offer_message, broadcast).await;
}

#[embassy_executor::task]
pub async fn web_service(){
    match WIFI_MODEL.lock().await.unwrap() {
        WifiModel::AP => {
            unsafe {
                if let Some(stack) = AP_STACK_MUT {
                    web_tcp_socket(stack).await;
                }
            }
        }
        WifiModel::STA => {
            unsafe {
                loop {
                    match use_wifi().await {
                        Ok(stack) => {
                            web_tcp_socket(stack).await;
                            finish_wifi().await;
                            break;
                        }
                        Err(_) => {}
                    }
                    Timer::after(Duration::from_millis(100));
                }
            }
        }
    }

}

async fn  web_tcp_socket<D: esp_wifi::wifi::WifiDeviceMode> (stack:&Stack<WifiDevice<'_,D>>){

    let mut rx_buffer = [0; 1536];
    let mut tx_buffer = [0; 1536];
    //网页配置服务
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
    loop {
        println!("Wait for connection...");
        let wait_stop = STOP_WEB_SERVICE.wait();
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 8080,
            })
            ;
        match select(wait_stop,r).await{
            Either::First(_) => {
                STOP_WEB_SERVICE.reset();
                break;
            }
            Either::Second(r) => {

                println!("Connected...");

                if let Err(e) = r {
                    println!("connect error: {:?}", e);
                    continue;
                }

                use embedded_io_async::Write;

                let mut buffer = [0u8; 1024];
                let mut pos = 0;
                loop {
                    match socket.read(&mut buffer).await {
                        Ok(0) => {
                            println!("read EOF");
                            break;
                        }
                        Ok(len) => {
                            let to_print =
                                unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

                            if to_print.contains("\r\n\r\n") {
                                print!("{}", to_print);
                                println!();

                                process_http(&mut socket,to_print).await;
                                break;
                            }

                            pos += len;
                        }
                        Err(e) => {
                            println!("read error: {:?}", e);
                            break;
                        }
                    };
                }

                let r = socket.flush().await;
                if let Err(e) = r {
                    println!("flush error: {:?}", e);
                }
                Timer::after(Duration::from_millis(1000)).await;

                socket.close();
                Timer::after(Duration::from_millis(1000)).await;

                socket.abort();

            }
        }
    }

}
async fn process_http(socket:&mut TcpSocket<'_>,buffer:&str){
    use embedded_io_async::Write;
    use heapless::String;

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    req.parse(buffer.as_ref());
    println!("request:{:?}",req);
    if let Some("GET") = req.method {
        if let Some("/config") = req.path {
            let content = concat!("HTTP/1.0 200 OK\r\n\r\n",include_str!("../files/config.html"));
            let r = socket
                .write_all(
                    content.as_bytes()
                )
                .await;

            if let Err(e) = r {
                println!("write error: {:?}", e);
            }
        }
    }
    if let Some("POST") = req.method {
        if let Some("/config") = req.path {
            let parts:heapless::Vec<&str,10> = buffer.split("\r\n\r\n").collect();
            if parts.len() > 1 {

                println!("body:{:?}",parts[1]);

                let form_fields:Vec<&str,10> = parts[1].split("&").collect();
                for form_field in form_fields {
                    let field:Vec<&str,2> = form_field.split("=").collect();
                    if field[0] == "ssid" {
                        println!("ssid:{}",field[1]);
                    }else if field[0] == "password" {
                        println!("password:{}",field[1]);
                    }
                }
                let r = socket
                    .write_all(
                        b"HTTP/1.0 200 OK\r\n\r\n\
            <html>\
                <body>\
                   <form action='/restart' method='POST'>\
                    <br/>\
                    <br/>\
                    <input type='submit' value='' />\
                   </form>\
                </body>\
            </html>\r\n",
                    )
                    .await;

                if let Err(e) = r {
                    println!("write error: {:?}", e);
                }
            }

        }
    }


}