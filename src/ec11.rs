use core::convert::Infallible;
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal::delay::DelayNs;

use embedded_hal_async::digital::Wait;
use esp_println::{print, println};
use hal::gpio::{AnyPin, Gpio0, Gpio1, Input, InputPin, PullUp};
use hal::prelude::_embedded_hal_digital_v2_InputPin;
use embassy_sync::channel::{Channel, Receiver, Sender};

use embedded_hal::digital::InputPin as OtherInputPin;
#[embassy_executor::task]
pub async fn detection(mut rx: Receiver<'static,CriticalSectionRawMutex,(bool, bool),64>)
{
    let mut last_a_state = false;
    let mut last_b_state = false;


    while let (is_a, state) = rx.receive().await {
        println!("a_state:{}",last_a_state);
        println!("b_state:{}",last_b_state);
        if is_a {
            last_a_state = state;
        } else {
            last_b_state = state;
        }

        if !last_a_state && !last_b_state {
            if last_a_state && !last_b_state {
                // 正转
                println!("Clockwise");
            } else if !last_a_state && last_b_state {
                // 反转
                println!("Counter-Clockwise");
            }
        }

        //Timer::after(Duration::from_millis(10)).await;  // 稍微延时以防抖
    }
}


#[embassy_executor::task]
pub async fn pin_a_task(mut pin :Gpio1<Input<PullUp>>, mut tx: Sender<'static,CriticalSectionRawMutex,(bool, bool),64>)
{
    // 初始化编码器状态
    loop {
        pin.wait_for_any_edge().await;
        let state = pin.is_high();
        tx.send((true, state.unwrap())).await;
    }
}


#[embassy_executor::task]
pub async fn pin_b_task(mut pin :Gpio0<Input<PullUp>>, mut tx: Sender<'static,CriticalSectionRawMutex,(bool, bool),64>)
{
    // 初始化编码器状态
    loop {
        pin.wait_for_any_edge().await;
        let state = pin.is_high();
        tx.send((false, state.unwrap())).await;
    }
}