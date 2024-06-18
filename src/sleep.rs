use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{ Duration, Instant};
use hal::peripherals::LPWR;
use hal::{Delay, Rtc};
use hal::gpio::{GpioPin, RTCPinWithResistors};
use hal::rtc_cntl::sleep::{RtcioWakeupSource, TimerWakeupSource, WakeSource, WakeupLevel};
use log::info;
use heapless::Vec;

use crate::CLOCKS_REF;
use crate::wifi::{force_stop_wifi, STOP_WIFI_SIGNAL};

pub static RTC_MANGE:Mutex<CriticalSectionRawMutex,Option<Rtc>> = Mutex::new(None);
pub static LAST_ACTIVE_TIME:Mutex<CriticalSectionRawMutex,Instant> = Mutex::new(Instant::MAX);
pub static mut WAKEUP_PINS:  Vec<(&'static mut dyn RTCPinWithResistors, WakeupLevel),5> = Vec::new();

pub async fn refresh_active_time(){
     *LAST_ACTIVE_TIME.lock().await = Instant::now();
}

pub async fn to_sleep(sleep_time:Duration,idle_time:Duration){
    if Instant::now().duration_since(*LAST_ACTIVE_TIME.lock().await) > idle_time  {
        //不关wifi,唤醒时运行到wifi部分会卡着
        force_stop_wifi().await;

        let wakeup_pins: &mut [(&mut dyn RTCPinWithResistors, WakeupLevel)] = unsafe{ WAKEUP_PINS.as_mut() };
        let rtcio = RtcioWakeupSource::new(wakeup_pins);

        let mut  wakeup_source =TimerWakeupSource::new(core::time::Duration::from_micros(sleep_time.as_micros()));

        let mut ws:Vec<& dyn WakeSource,2> = Vec::new();
        ws.push(&rtcio);
        if sleep_time.as_ticks() > 0{
            ws.push(&wakeup_source);
        }

        let mut delay = Delay::new(unsafe{CLOCKS_REF.unwrap()});
        RTC_MANGE.lock().await.as_mut().unwrap().sleep_deep(ws.as_slice(), &mut delay);

    }
}


pub async fn add_rtcio(rtcpin:&'static mut dyn RTCPinWithResistors, wakeup_level: WakeupLevel){
    unsafe {
        WAKEUP_PINS.push((rtcpin,wakeup_level));
    }
}