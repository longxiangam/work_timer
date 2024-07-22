#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(static_mut_refs)]
#![feature(async_closure)]
#![allow(unused)]
#![allow(async_fn_in_trait)]
#![feature(generic_const_exprs)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;
pub mod display;
pub mod ec11;
pub mod widgets;
pub mod pages;
pub mod wifi;
pub mod event;
pub mod chip8;
pub mod sound;
mod request;
mod worldtime;
mod random;
mod model;
mod weather;
mod storage;
pub mod web_service;
pub mod sleep;
pub mod battery;

use alloc::string::ToString;
use core::convert::Infallible;
use core::{mem, ptr};
use core::mem::size_of;
use core::str::FromStr;

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Ipv4Address, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;


use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::InputPin;
use esp_wifi::wifi::{AuthMethod, ClientConfiguration, Configuration, WifiError};
use esp_backtrace as _;
use esp_println::{print, println};
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::{initialize, EspWifiInitFor};
use hal::clock::{Clock, ClockControl, Clocks};
use hal::{Cpu, rng::Rng};
use hal::{ peripherals::Peripherals, prelude::*, dma_descriptors,
          spi::{ master::{prelude::*, Spi}, SpiMode,  }, dma::*};
use hal::analog::adc::{Adc, AdcConfig, Attenuation};
use hal::peripherals::{ADC1, LPWR, SPI2, WIFI};
use static_cell::{make_static, StaticCell};
use hal::dma::{DmaDescriptor, DmaPriority};
use hal::dma::Channel0;


use hal::gpio::{Gpio11, Gpio12, Gpio13, Gpio18, Gpio19, Gpio4, Gpio5, Gpio8, Gpio9, Input, NO_PIN, OutputOpenDrain, Output, Io, Level, Pull, Analog};
use hal::ledc::{channel,  LowSpeed, LSGlobalClkSource, timer};
use hal::ledc::timer::config::Duty::Duty8Bit;
use hal::peripheral::Peripheral;
use hal::reset::software_reset;
use hal::riscv::_export::critical_section::Mutex;
use hal::riscv::_export::critical_section;
use hal::rtc_cntl::{get_reset_reason, get_wakeup_cause, Rtc, SocResetReason};
use hal::rtc_cntl::sleep::{RtcioWakeupSource, TimerWakeupSource, WakeupLevel};

use hal::spi::FullDuplexMode;
use hal::spi::master::dma::SpiDma;
use hal::system::Peripheral::Ledc;
use hal::system::SystemControl;
use hal::timer::OneShotTimer;
use hal::timer::timg::TimerGroup;
use heapless::String;
use lcd_drivers::color::TwoBitColor;
use log::info;
use crate::battery::Battery;

use crate::pages::{ Page};
use crate::pages::init_page::InitPage;
use crate::sleep::{add_rtcio, refresh_active_time, RTC_MANGE, to_sleep, WAKEUP_PINS};
use crate::sound::{buzzer_task, PWM_PLAYER, PwmPlayer, SoundType};
use crate::storage::{enter_process, NvsStorage, read_flash, WIFI_INFO, WifiStorage, write_flash};
use crate::weather::weather_worker;
use crate::wifi::{connect_wifi, REINIT_WIFI_SIGNAL, start_wifi_ap, WIFI_MODEL, WifiModel};
use crate::worldtime::{get_clock, ntp_worker};




const DESCRIPTORS_SIZE: usize = 8 * 3;
/// Descriptors for SPI DMA
static DESCRIPTORS: StaticCell<[DmaDescriptor; DESCRIPTORS_SIZE]> = StaticCell::new();

/// RX descriptors for SPI DMA
static RX_DESCRIPTORS: StaticCell<[DmaDescriptor; DESCRIPTORS_SIZE]> = StaticCell::new();
static CHANNEL: Channel<CriticalSectionRawMutex, (bool,bool), 64> = Channel::new();
pub static mut CLOCKS_REF: Option<&'static Clocks>  =  None;

#[main]
async fn main(spawner: Spawner) {
    alloc();
    if let Err(error) = main_fallible(&spawner).await {
        println!("Error while running firmware: {error:?}");
    }
/*    if let Err(error) = main_fallible(&spawner).await {
        println!("Error while running firmware: {error:?}");
    }*/
}

/*async fn test_main(spawner: &Spawner)->Result<(),Error>{
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks  = &*make_static!( ClockControl::max(system.clock_control).freeze());

    let mut rtc = Rtc::new(peripherals.LPWR,None);
    RTC_MANGE.lock().await.replace(rtc);
    unsafe {
        CLOCKS_REF.replace(clocks);
    }
    let reason = get_reset_reason(Cpu::ProCpu).unwrap_or(SocResetReason::ChipPowerOn);
    println!("reset reason: {:?}", reason);
    let wake_reason = get_wakeup_cause();
    println!("wake reason: {:?}", wake_reason);

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks, None);
    let timers:&mut [OneShotTimer<ErasedTimer>; 1] =  make_static!([OneShotTimer::new(timg0.timer0.into())]);
    let timers = mk_static!(
        [OneShotTimer<ErasedTimer>; 1],
        [OneShotTimer::new(timg0.timer0.into())]
    );
    esp_hal_embassy::init(&clocks, timers);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);
    loop {
        println!("enter test");
        Timer::after_secs(1);
    }
}*/


async fn main_fallible(spawner: &Spawner)->Result<(),Error>{

    let peripherals = Peripherals::take();

    let mut system = SystemControl::new(peripherals.SYSTEM);
    let clocks  = &*make_static!( ClockControl::max(system.clock_control.clone_unchecked()).freeze());

    let mut rtc = Rtc::new(peripherals.LPWR,None);
    RTC_MANGE.lock().await.replace(rtc);
    unsafe {
        CLOCKS_REF.replace(clocks);
    }
    let reason = get_reset_reason(Cpu::ProCpu).unwrap_or(SocResetReason::ChipPowerOn);
    println!("reset reason: {:?}", reason);
    let wake_reason = get_wakeup_cause();
    println!("wake reason: {:?}", wake_reason);

    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timg0);
/*    //测试软件重启位
    unsafe {
        peripherals.LPWR.options0().modify(|_, w| w.sw_sys_rst().set_bit());
    }*/

    println!("do main");
    enter_process().await;
    //spi
    let mut io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut buzzer =Output::new(io.pins.gpio13,Level::High);
    buzzer.set_low();

    let mut pwm_player =  PwmPlayer::init(peripherals.LEDC, &clocks, buzzer);
    unsafe {
        PWM_PLAYER.replace(pwm_player);
    }

    spawner.spawn(buzzer_task()).ok();


    let epd_sclk = io.pins.gpio6;
    let epd_mosi = io.pins.gpio7;
    let epd_cs = io.pins.gpio2;
    let epd_rst = io.pins.gpio10;
    let epd_dc = io.pins.gpio3;

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    let descriptors: &'static mut _ = DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);
    let rx_descriptors: &'static mut _ =
        RX_DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);



    let spi_bus = Spi::new(peripherals.SPI2, 48_u32.MHz(), SpiMode::Mode0, &clocks)
        .with_sck(epd_sclk)
        .with_mosi(epd_mosi);


    //: SpiDma<'_, SPI2, Channel0, FullDuplexMode,SpiMode>
    let spi_dma = spi_bus.with_dma(
        dma_channel.configure(false, descriptors, rx_descriptors, DmaPriority::Priority0),
    );

    spawner.spawn(crate::display::render(spi_dma,epd_cs,epd_rst,epd_dc)).ok();

    let mut init_page = InitPage::new();


    let mut a_point = Input::new(unsafe{io.pins.gpio1.clone_unchecked()},Pull::Up);
    let mut b_point = Input::new(unsafe{io.pins.gpio0.clone_unchecked()},Pull::Up);

    let mut key1 = Input::new( io.pins.gpio11,Pull::Up);
    let key2 = Input::new( io.pins.gpio8,Pull::Up);
    let key3 = Input::new( io.pins.gpio9,Pull::Up);
    let mut key_ec11 = Input::new(unsafe{ io.pins.gpio5.clone_unchecked()},Pull::Up);
    let mut bat_adc = io.pins.gpio4;


    let rtc_io_2 = make_static!( io.pins.gpio5);
    let rtc_io_a = make_static!( io.pins.gpio1);
    let rtc_io_b = make_static!( io.pins.gpio0);

    add_rtcio( rtc_io_2,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_a,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_b,  WakeupLevel::Low).await;

    spawner.spawn(ec11::task(a_point,b_point,key_ec11)).ok();


    spawner.spawn(event::run(key1,key2,key3)).ok();

    //type AdcCal = hal::analog::adc::AdcCalBasic<hal::peripherals::ADC1>;
    //type AdcCal = hal::analog::adc::AdcCalLine<ADC1>;
    type AdcCal = hal::analog::adc::AdcCalCurve<ADC1>;

    let mut adc1_config = AdcConfig::new();
    let mut adc1_pin =
        adc1_config.enable_pin_with_cal::<_, AdcCal>(bat_adc, Attenuation::Attenuation11dB);
    let adc1 = Adc::new(peripherals.ADC1, adc1_config);
    let battery = Battery::new(adc1);
    battery::BATTERY.lock().await.replace(battery);
    battery::ADC_PIN.lock().await.replace(adc1_pin);
    spawner.spawn(crate::battery::test_bat_adc()).ok();
    //连接wifi
    let mut need_ap = false;
    refresh_active_time().await;



    loop {
        if let Some(wifi) = WIFI_INFO.lock().await.as_ref(){
            println!("wifi_state:{:?}",wifi);
            if !wifi.wifi_finish {
                need_ap = true;
            }
            break;
        }
        Timer::after(Duration::from_millis(50)).await;
    }
    if  need_ap {
        WIFI_MODEL.lock().await.replace(WifiModel::AP);
        println!("wifi_model:{:?}",WIFI_MODEL.lock().await.as_ref());
        let stack = start_wifi_ap(spawner,
                                 peripherals.SYSTIMER,
                                 Rng::new(peripherals.RNG),
                                 peripherals.WIFI,
                                 unsafe {system.clock_control.clone_unchecked()},
                                 clocks).await;

        loop {
            let mut qrcode_page = pages::setting_page::SettingPage::new();
            qrcode_page.bind_event().await;
            qrcode_page.run(spawner.clone()).await;
            Timer::after(Duration::from_secs(50)).await;
        }

    }else {
        //init_page.append_log("正在连接wifi").await;
        Timer::after_millis(10).await;
        WIFI_MODEL.lock().await.replace(WifiModel::STA);


        spawner.spawn(ntp_worker()).ok();
        spawner.spawn(weather_worker()).ok();

        //init_page.run(spawner.clone()).await;

        spawner.spawn(pages::main_task(spawner.clone())).ok();

        let stack = connect_wifi(spawner,
                                 peripherals.SYSTIMER,
                                 Rng::new(peripherals.RNG),
                                 peripherals.WIFI,
                                 peripherals.RADIO_CLK,
                                 clocks).await;
        //init_page.append_log("已连接wifi").await;
    }




    loop {
        if let Some(clock) =  get_clock(){
            println!("Current_time: {}", clock.get_date_str().await);
        }
        Timer::after(Duration::from_secs(10)).await;

    }
}



fn alloc(){
    // -------- Setup Allocator --------
    const HEAP_SIZE: usize = 60 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    #[global_allocator]
    static ALLOCATOR: embedded_alloc::Heap = embedded_alloc::Heap::empty();
    unsafe { ALLOCATOR.init(&mut HEAP as *const u8 as usize, core::mem::size_of_val(&HEAP)) };
}


pub fn enter_deep(rtc_cntl: LPWR, mut delay: hal::delay::Delay, interval: core::time::Duration) -> ! {
    let wakeup_source = TimerWakeupSource::new(interval);

    let mut rtc = Rtc::new(rtc_cntl,None);


    info!("Entering deep sleep for {interval:?}");
    rtc.sleep_deep(&[&wakeup_source], &mut delay);
}



/// An error
#[derive(Debug)]
enum Error {
    /// An impossible error existing only to satisfy the type system
    Impossible(Infallible),

    /// Error while parsing SSID or password
    ParseCredentials,

    /// An error within WiFi operations
    #[allow(unused)]
    Wifi(WifiError),


}