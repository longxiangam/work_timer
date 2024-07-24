#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(generic_const_exprs)]

#![allow(static_mut_refs)]

extern crate alloc;

mod display;
mod wifi;
mod random;
mod storage;
mod ec11;
mod event;
/*mod sound;*/
mod battery;
mod utils;
mod sleep;
mod model;
mod request;
mod weather;
mod worldtime;
mod web_service;
mod chip8;
mod widgets;
mod pages;


use core::convert::Infallible;
use esp_backtrace as _;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_println::println;
use esp_wifi::wifi::WifiError;
use hal::clock::{ClockControl, Clocks};
use hal::dma::{Dma, DmaDescriptor};
use hal::prelude::{_fugit_RateExtU32, main};
use hal::{Cpu, entry};
use hal::analog::adc::{Adc, AdcConfig, Attenuation};
use hal::gpio::{Gpio0, Gpio1, Gpio5, Io, Level, Output};
use hal::peripheral::Peripheral;
use hal::peripherals::{ADC1, Peripherals};
use hal::reset::{get_reset_reason, get_wakeup_cause};
use hal::rtc_cntl::{Rtc, SocResetReason};
use hal::spi::master::Spi;

use hal::spi::SpiMode;
use hal::spi::master::prelude::*;
use hal::dma::*;
use hal::rng::Rng;
use hal::rtc_cntl::sleep::WakeupLevel;
use hal::system::SystemControl;
use hal::timer::{ErasedTimer, OneShotTimer};
use hal::timer::timg::TimerGroup;
use static_cell::{ StaticCell};
use crate::battery::Battery;
use crate::pages::init_page::InitPage;
use crate::pages::Page;
use crate::sleep::{add_rtcio, refresh_active_time, RTC_MANGE};
use crate::storage::{enter_process, init_storage_area, WIFI_INFO};
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

async fn main_fallible(spawner: &Spawner)->Result<(),Error> {
    let mut peripherals = Peripherals::take();

    let mut system = SystemControl::new(unsafe{peripherals.SYSTEM.clone_unchecked()});

    let clocks_val = ClockControl::max(system.clock_control).freeze();
    let clocks  = make_static!(Clocks,clocks_val);

    let mut rtc = Rtc::new(peripherals.LPWR,None);
    RTC_MANGE.lock().await.replace(rtc);
    unsafe {
        CLOCKS_REF.replace(clocks);
    }
    let reason = get_reset_reason().unwrap_or(SocResetReason::ChipPowerOn);
    println!("reset reason: {:?}", reason);
    let wake_reason = get_wakeup_cause();
    println!("wake reason: {:?}", wake_reason);
/*    let timg0 = TimerGroup::new(unsafe{peripherals.TIMG0.clone_unchecked()}, &clocks, None);

    let timers = make_static!(
        [OneShotTimer<ErasedTimer>; 1],
        [OneShotTimer::new(timg0.timer0.into())]
    );*/

    let systimer = hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    let timers = make_static!(
        [OneShotTimer<ErasedTimer>; 1],
        [OneShotTimer::new(systimer.alarm0.into())]
    );
    esp_hal_embassy::init(&clocks, timers);
    println!("do main");

    enter_process().await;

    let mut io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let epd_sclk = io.pins.gpio6;
    let epd_mosi = io.pins.gpio7;
    let epd_cs = io.pins.gpio2;
    let epd_rst =  io.pins.gpio10;
    let epd_dc = io.pins.gpio3;

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    let descriptors: &'static mut _ = DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);
    let rx_descriptors: &'static mut _ =
        RX_DESCRIPTORS.init([DmaDescriptor::EMPTY; DESCRIPTORS_SIZE]);



    let spi_bus = Spi::new(peripherals.SPI2, 48_u32.MHz(), SpiMode::Mode0, &clocks)
        .with_sck(epd_sclk)
        .with_mosi(epd_mosi);



    //:  SpiDma<'static, SPI2, Channel0, FullDuplexMode,Async>
    let spi_dma = spi_bus.with_dma(
        dma_channel.configure_for_async(false, DmaPriority::Priority0),
        descriptors,
        rx_descriptors,
    );
    spawner.spawn(crate::display::render(spi_dma,epd_cs,epd_rst,epd_dc)).ok();
    let mut init_page = InitPage::new();



    let mut a_point = unsafe{io.pins.gpio1.clone_unchecked()};
    let mut b_point = unsafe{io.pins.gpio0.clone_unchecked()};

    let mut key1 =  io.pins.gpio11;
    let key2 = io.pins.gpio8;
    let key3 =  io.pins.gpio9;
    let mut key_ec11 = unsafe{ io.pins.gpio5.clone_unchecked()};
    let mut bat_adc = io.pins.gpio4;


    let rtc_io_2 = make_static!(Gpio5, io.pins.gpio5);
    let rtc_io_a = make_static!(Gpio1, io.pins.gpio1);
    let rtc_io_b = make_static!(Gpio0, io.pins.gpio0);

    add_rtcio( rtc_io_2,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_a,  WakeupLevel::Low).await;
    add_rtcio( rtc_io_b,  WakeupLevel::Low).await;

    spawner.spawn(ec11::task(a_point,b_point,key_ec11)).ok();

    spawner.spawn(event::run(key1,key2,key3)).ok();


    type AdcCal = hal::analog::adc::AdcCalCurve<ADC1>;

    let mut adc1_config = AdcConfig::new();
    let mut adc1_pin =
        adc1_config.enable_pin_with_cal::<_, AdcCal>(bat_adc, Attenuation::Attenuation11dB);
    let adc1 = Adc::new(peripherals.ADC1, adc1_config);
    let battery = Battery::new(adc1);
    battery::BATTERY.lock().await.replace(battery);
    battery::ADC_PIN.lock().await.replace(adc1_pin);
  /*  spawner.spawn(crate::battery::test_bat_adc()).ok();*/
    //连接wifi
    let mut need_ap = false;
    refresh_active_time().await;

    println!("start wifi");

  /*  loop {
        println!("entry need_ap 1");
        if let Some(wifi) = WIFI_INFO.lock().await.as_ref(){
            println!("wifi_finish:{:?}",wifi.wifi_finish);
            println!("wifi_ssid:{:?}",wifi.wifi_ssid);
            //println!("wifi_password:{:?}",wifi.wifi_password);
            if !wifi.wifi_finish {
                need_ap = true;
            }
            println!("entry need_ap 2");
            break;
        }
        println!("entry need_ap");
        Timer::after(Duration::from_millis(50)).await;
    }*/
    println!("entry need_ap");
    if  need_ap {
        println!("entry ap");
        WIFI_MODEL.lock().await.replace(WifiModel::AP);
        println!("wifi_model:{:?}",WIFI_MODEL.lock().await.as_ref());
        let stack = start_wifi_ap(spawner,
                                  peripherals.TIMG0,
                                  Rng::new(peripherals.RNG),
                                  peripherals.WIFI,
                                  peripherals.RADIO_CLK,
                                  clocks).await;

        loop {
            let mut qrcode_page = pages::setting_page::SettingPage::new();
            qrcode_page.bind_event().await;
            qrcode_page.run(spawner.clone()).await;
            Timer::after(Duration::from_secs(50)).await;
        }

    }else {
        println!("entry sta");
        //init_page.append_log("正在连接wifi").await;
        Timer::after_millis(10).await;
        WIFI_MODEL.lock().await.replace(WifiModel::STA);
        spawner.spawn(weather_worker()).ok();

        spawner.spawn(ntp_worker()).ok();

        spawner.spawn(pages::main_task(spawner.clone())).ok();

        let stack = connect_wifi(spawner,
                                 peripherals.TIMG0,
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
#[embassy_executor::task]
pub async fn  do_loop(num:u32){
    loop{
        println!("do_loop,{}",num);
        Timer::after(Duration::from_secs(1)).await;
    }

}
#[embassy_executor::task]
pub async fn  do_loop2(num:u32){
    loop{
        println!("do_loop,{}",num);
        Timer::after(Duration::from_secs(1)).await;
    }

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

fn alloc(){
    // -------- Setup Allocator --------
    const HEAP_SIZE: usize = 60 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    #[global_allocator]
    static ALLOCATOR: embedded_alloc::Heap = embedded_alloc::Heap::empty();
    unsafe { ALLOCATOR.init(&mut HEAP as *const u8 as usize, core::mem::size_of_val(&HEAP)) };
}
