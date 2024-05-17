use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::future::Future;
use embassy_futures::select::{Either, Either4, select, Select, select4};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::InputPin;
use embedded_hal_async::digital::Wait;
use esp_println::println;
use hal::gpio::{Gpio11, Gpio5, Gpio8, Gpio9, Input, PullUp};
use hal::prelude::_embedded_hal_digital_v2_InputPin;

use crate::pages::Page;

#[derive(Eq, PartialEq,Copy,Clone,Debug)]
pub enum EventType{
    KeyShort(u32),
    KeyLongStart(u32),
    KeyLongEnd(u32),
    KeyDouble(u32),
    WheelBack,
    WheelFront,
}
pub struct EventInfo{
}


struct Listener{
    callback:Box<dyn FnMut() +  Send + Sync>,
    event_type:EventType
}

static LISTENER:Mutex<CriticalSectionRawMutex,Vec<Listener>>  = Mutex::new(vec![]) ;
pub async fn on<F>(event_type: EventType, callback: F)
where F: FnMut() + Send + Sync + 'static,
{

    LISTENER.lock().await.push(Listener{callback:Box::new(callback),event_type});
}

pub async fn un<F>(event_type: EventType)
    where F: FnMut(),
{
    let mut vec = LISTENER.lock().await;

    let mut find_index:Option<usize> = None;
    for (index,listener) in vec.iter().enumerate() {
        if listener.event_type == event_type{
            find_index = Some(index);
        }
    }
    if let Some(v) = find_index {
        vec.remove(v);
    }
}

pub async fn clear(){
    let mut vec = LISTENER.lock().await;
    vec.clear();
}


pub async fn toggle_event(event_type: EventType,ms:u64){
    println!("event_type:{:?}",event_type);


}


#[embassy_executor::task]
pub async  fn run(mut key1:Gpio11<Input<PullUp>>,mut key2:Gpio5<Input<PullUp>>,
                  mut key3:Gpio8<Input<PullUp>>,mut key4:Gpio9<Input<PullUp>>, ){
    loop {

        let key1_edge = key1.wait_for_falling_edge();
        let key2_edge = key2.wait_for_falling_edge();
        let key3_edge = key3.wait_for_falling_edge();
        let key4_edge = key4.wait_for_falling_edge();
        match  select4(key1_edge,key2_edge,key3_edge,key4_edge).await {
            Either4::First(_) => {
                key_detection::<_,1>(&mut key1).await;
            }
            Either4::Second(_) => {
                key_detection::<_,2>(&mut key2).await;
            }
            Either4::Third(_) => {
                key_detection::<_,3>(&mut key3).await;
            }
            Either4::Fourth(_) => {
                key_detection::<_,4>(&mut key4).await;
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}

async fn key_detection<P,const Num:usize>(key: &mut P)
where P:InputPin
{
    let begin_ms = Instant::now().as_millis();
    let mut is_long = false;
    loop {
        let mut is_low_times = 0;
        for i in 0..10 {
            if key.is_low().unwrap() {
                is_low_times += 1;
            }
        }
        if is_low_times > 8 {
            //按下
            let current = Instant::now().as_millis();
            if current - begin_ms > 500 {
                //长时间按下
                if !is_long {
                    is_long = true;
                    toggle_event(EventType::KeyLongStart(Num as u32), current).await;
                }
            }
        } else if is_low_times < 2 {
            //释放
            let current = Instant::now().as_millis();
            if is_long {
                //长时间按下后释放
                toggle_event(EventType::KeyLongEnd(Num as u32), current).await;
                return;
            } else {
                //短时按下，等几ms 看是否有下一次按下，如有则是双击

                loop {
                    let current = Instant::now().as_millis();
                    if current - begin_ms > 400 {
                        toggle_event(EventType::KeyShort(Num as u32), current).await;
                        return;
                    }
                    let mut is_low_times = 0;
                    for i in 0..10 {
                        if key.is_low().unwrap() {
                            is_low_times += 1;
                        }
                    }

                    //变低
                    if is_low_times > 8{
                        toggle_event(EventType::KeyDouble(Num as u32), current).await;
                        return;
                    }
                }

            }
        }
        Timer::after(Duration::from_millis(1)).await;
    }
}
