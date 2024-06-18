#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(static_mut_refs)]
#![feature(async_closure)]
#![allow(unused)]
#![allow(async_fn_in_trait)]
#![feature(generic_const_exprs)]

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
mod sleep;

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
use hal::{Cpu, Rng, Rtc};
use hal::{embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup,dma_descriptors,
          spi::{ master::{prelude::*, Spi}, SpiMode,  }, dma::Dma,gpio::IO};
use hal::adc::{AdcConfig, Attenuation,ADC};

use hal::peripherals::{ADC1, LPWR, SPI2, WIFI};
use static_cell::{make_static, StaticCell};
use hal::dma::{DmaDescriptor, DmaPriority};
use hal::dma::Channel0;

use hal::gpio::{Gpio11, Gpio12, Gpio13, Gpio18, Gpio19, Gpio4, Gpio5, Gpio8, Gpio9, Input, NO_PIN, OpenDrain, Output, PullUp, RTCPinWithResistors};
use hal::ledc::{channel, LEDC, LowSpeed, LSGlobalClkSource, timer};
use hal::peripheral::Peripheral;
use hal::reset::software_reset;
use hal::riscv::_export::critical_section::Mutex;
use hal::riscv::_export::critical_section;
use hal::rtc_cntl::{get_reset_reason, get_wakeup_cause, SocResetReason};
use hal::rtc_cntl::sleep::{RtcioWakeupSource, TimerWakeupSource, WakeupLevel};

use hal::spi::FullDuplexMode;
use hal::spi::master::dma::SpiDma;
use hal::system::Peripheral::Ledc;
use hal::system::RadioClockControl;
use heapless::String;
use lcd_drivers::color::TwoBitColor;
use log::info;

use crate::pages::{ Page};
use crate::pages::init_page::InitPage;
use crate::sleep::{add_rtcio, refresh_active_time, RTC_MANGE, to_sleep, WAKEUP_PINS};
use crate::storage::{enter_process, NvsStorage, read_flash, WIFI_INFO, WifiStorage, write_flash};
use crate::weather::weather_worker;
use crate::wifi::{connect_wifi, start_wifi_ap, WIFI_MODEL, WifiModel};
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
}



async fn main_fallible(spawner: &Spawner)->Result<(),Error>{

    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks  = &*make_static!( ClockControl::max(system.clock_control).freeze());

    let mut rtc = Rtc::new(peripherals.LPWR);
    RTC_MANGE.lock().await.replace(rtc);
    unsafe {
        CLOCKS_REF.replace(clocks);
    }
    let reason = get_reset_reason(Cpu::ProCpu).unwrap_or(SocResetReason::ChipPowerOn);
    println!("reset reason: {:?}", reason);
    let wake_reason = get_wakeup_cause();
    println!("wake reason: {:?}", wake_reason);

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

/*    //测试软件重启位
    unsafe {
        peripherals.LPWR.options0().modify(|_, w| w.sw_sys_rst().set_bit());
    }*/

    println!("do main");
    enter_process().await;
    //spi
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let epd_sclk = io.pins.gpio6;
    let epd_mosi = io.pins.gpio7;
    let epd_cs = io.pins.gpio2.into_push_pull_output();
    let epd_rst = io.pins.gpio10.into_push_pull_output();
    let epd_dc = io.pins.gpio3.into_push_pull_output();

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    let descriptors: &'static mut _ = DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);
    let rx_descriptors: &'static mut _ =
        RX_DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);



    let spi_bus = Spi::new(peripherals.SPI2, 48_u32.MHz(), SpiMode::Mode0, &clocks)
        .with_sck(epd_sclk)
        .with_mosi(epd_mosi);


    let spi_dma: SpiDma<'_, SPI2, Channel0, FullDuplexMode> = spi_bus.with_dma(
        dma_channel.configure(false, descriptors, rx_descriptors, DmaPriority::Priority0),
    );

    spawner.spawn(crate::display::render(spi_dma,epd_cs,epd_rst,epd_dc)).ok();

    let mut init_page = InitPage::new();


    let mut a_point = io.pins.gpio1.into_pull_up_input();
    let mut b_point = io.pins.gpio0.into_pull_up_input();

    let mut key1 = io.pins.gpio11.into_pull_up_input();
    let mut key2 = io.pins.gpio5.into_pull_up_input();
    let key3 = io.pins.gpio8.into_pull_up_input();
    let key4 = io.pins.gpio9.into_pull_up_input();
    let key_ec11 = io.pins.gpio13.into_pull_up_input();

    let rtc_io_2 = make_static!(unsafe{ key2.clone_unchecked()});
    let rtc_io_a = make_static!(unsafe{ a_point.clone_unchecked()});
    let rtc_io_b = make_static!(unsafe{ b_point.clone_unchecked()});

    add_rtcio( rtc_io_2,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_a,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_b,  WakeupLevel::Low).await;

    spawner.spawn(ec11::task(a_point,b_point,key_ec11)).ok();


    spawner.spawn(event::run(key1,key2,key3,key4)).ok();


    //连接wifi
    let mut need_ap = false;
    refresh_active_time().await;
/*    //测试sleep
    loop {

        to_sleep(Duration::from_secs(0),Duration::from_secs(5)).await;

        Timer::after(Duration::from_secs(1)).await;
    }*/



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
                                 system.radio_clock_control,
                                 clocks).await;

        loop {
            let mut qrcode_page = pages::setting_page::SettingPage::new();
            qrcode_page.bind_event().await;
            qrcode_page.run(spawner.clone()).await;
            Timer::after(Duration::from_secs(50)).await;
        }

    }else {
        init_page.append_log("正在连接wifi").await;
        WIFI_MODEL.lock().await.replace(WifiModel::STA);
        let stack = connect_wifi(spawner,
                                 peripherals.SYSTIMER,
                                 Rng::new(peripherals.RNG),
                                 peripherals.WIFI,
                                 system.radio_clock_control,
                                 clocks).await;
        init_page.append_log("已连接wifi").await;
        spawner.spawn(ntp_worker()).ok();
        spawner.spawn(weather_worker()).ok();

        //init_page.run(spawner.clone()).await;

        spawner.spawn(pages::main_task(spawner.clone())).ok();
    }


    loop {
        Timer::after_secs(100);
    }

  /*  loop {
        if let Some(clock) =  get_clock(){
            println!("Current_time: {}", clock.get_date_str().await);
        }
        //enter_deep(peripherals.LPWR, hal::Delay::new(clocks), core::time::Duration::from_secs(10));
        Timer::after(Duration::from_secs(10)).await;

    }*/
}



fn alloc(){
    // -------- Setup Allocator --------
    const HEAP_SIZE: usize = 60 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    #[global_allocator]
    static ALLOCATOR: embedded_alloc::Heap = embedded_alloc::Heap::empty();
    unsafe { ALLOCATOR.init(&mut HEAP as *const u8 as usize, core::mem::size_of_val(&HEAP)) };
}


pub fn enter_deep(rtc_cntl: LPWR, mut delay: hal::Delay, interval: core::time::Duration) -> ! {
    let wakeup_source = TimerWakeupSource::new(interval);

    let mut rtc = Rtc::new(rtc_cntl);


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