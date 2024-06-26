use core::cmp::max;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use esp_println::println;
use hal::adc::{ADC, AdcCalBasic, AdcPin};
use hal::gpio::{Analog, GpioPin};
use hal::peripherals::ADC1;
use hal::prelude::_embedded_hal_adc_OneShot;

type AdcCal = hal::analog::adc::AdcCalCurve<ADC1>;
pub struct Battery<'d>{
    adc:ADC<'d,ADC1>,
    pub voltage:u32,
    pub percent:u32,
}

impl <'d> Battery<'d>{
    pub fn new(adc:ADC<'d,ADC1>)->Battery{
        Self{
            adc,
            voltage: 0,
            percent: 0,
        }
    }
}

pub static BATTERY:Mutex<CriticalSectionRawMutex,Option<Battery>> = Mutex::new(None);
pub static ADC_PIN:Mutex<CriticalSectionRawMutex,Option<AdcPin<GpioPin<Analog,4>,ADC1,AdcCal>>> = Mutex::new(None);

#[task]
pub async fn test_bat_adc(){

    loop {
        if let Some(v) = BATTERY.lock().await.as_mut(){
            if let Some(pin) =  ADC_PIN.lock().await.as_mut(){
                let val = v.adc.read(pin);
                match val {
                    Ok(adc_value) => {
                        v.voltage = adc_value as u32 * 2 ;
                        let max =  4100;
                        let min = 3200;
                        let current_v = max.min(v.voltage);
                        v.percent = (current_v-3200)*100 / (4100 - 3200) ;
                        println!("current_v:{:?}",current_v);
                        println!("电量:{:?}",v.voltage);
                        println!("百分比:{:?}", v.percent);
                    }
                    Err(e) => {
                        println!("error:{:?}",e);
                    }
                }
            }
        }

        Timer::after_secs(6).await;
    }
}