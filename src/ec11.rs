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
use esp_wifi::wifi::is_from_isr;
use crate::display;
use crate::display::{display_mut, RenderInfo};
use crate::ec11::WheelDirection::{BACK, FRONT, NO_STATE};
use crate::event::{EventType, toggle_event};

#[derive(Eq, PartialEq)]
enum WheelDirection{
    FRONT,
    BACK,
    NO_STATE,
}
const SAMPLE_TIMES:u32 = 10;
const JUDGE_TIMES:u32 = 8;

#[embassy_executor::task]
pub async fn task(mut a_point :Gpio1<Input<PullUp>>,mut b_point :Gpio0<Input<PullUp>>){
    // 初始化编码器状态

    let mut begin_state = WheelDirection::NO_STATE;
    let mainPageSender =  crate::pages::main_page::MAIN_PAGE_CHANNEL.sender();


    // 开始监听编码器状态变化

    loop {

        a_point.wait_for_any_edge().await;

        let mut a_is_low_times = 0;
        let mut b_is_low_times = 0;
        for i in 0..SAMPLE_TIMES {
            if a_point.is_low().unwrap() {
                a_is_low_times += 1;
            }
            if b_point.is_low().unwrap() {
                b_is_low_times += 1;
            }
        }

        let mut a_is_down = false;
        let mut b_is_down = false;
        if(a_is_low_times > JUDGE_TIMES){
            a_is_down = true;
        }else if a_is_low_times < SAMPLE_TIMES - JUDGE_TIMES {
            a_is_down = false;
        }else {
            continue;
        }
        if b_is_low_times > JUDGE_TIMES {
            b_is_down = true;
        }else if b_is_low_times < SAMPLE_TIMES - JUDGE_TIMES {
            b_is_down = false;
        }else {
            continue;
        }
        //下降沿开始
        if(a_is_down){
            if(b_is_down){
                begin_state = FRONT;
                continue;
            }else if(!b_is_down){
                begin_state =  BACK;
                continue;
            }
            begin_state = NO_STATE;

        }else{
            //上升沿判断结束
            if(!b_is_down){
                if begin_state == FRONT  {
                    toggle_event(EventType::WheelFront,Instant::now().as_millis()).await;
                }
            }else if(b_is_down){
                if begin_state == BACK {
                    toggle_event(EventType::WheelFront,Instant::now().as_millis()).await;
                }
            }
            begin_state = NO_STATE;
        }
    }
}


fn detection(mut a_point :Gpio1<Input<PullUp>>, mut b_point :Gpio0<Input<PullUp>>) -> Option<WheelDirection> {
    // 检查状态变化，确定旋转方向
    /***
AL a 低   AH  a 高   BL b 低  BH b 高
逻辑判断，以反转为例，（正反转只是当a 或 b 拉高时，当前的a b电位状态相反）：
1.未触发时 AH BH 不进入逻辑
2.开始反转 AL BH 进入逻辑，上次状态为 AH BH， 更新后上次状态为 AL BH
3.一段时间的无状态变化不进入逻辑，此时状态为 AL BH
4.b 进入拉低状态 当前状态为 AL BL，上次状态为 AL BH ， 更新后上次状态为 AL BL
5.一段时间的无状态变化不进入逻辑，此时状态为 AL BL
6.a 进入拉高状态 当前状态为 AH BL, 上次状态 为 AL BL  进入方向判断逻辑 ，
    如果是正转，此时 状态为 AL BH
    如果是反转，此时 状态为 AH BL
 */
    // 初始化编码器状态
    let mut num = 0;
    let mut last_a_state = a_point.is_low();
    let mut last_b_state = b_point.is_low();
     let mut current_a_state = a_point.is_low();
     let mut current_b_state = b_point.is_low();
     if current_a_state != last_a_state || current_b_state != last_b_state {
         if last_a_state.unwrap() && last_b_state.unwrap() {
             if current_a_state.unwrap() && !current_b_state.unwrap() {
                 // 正转
                 num = num + 1;
                 println!("正转");
                 println!("num :{}", num);
                 return Some( WheelDirection::FRONT);
             } else if !current_a_state.unwrap() && current_b_state.unwrap() {
                 // 反转
                 num = num - 1;
                 println!("反转");
                 println!("num :{}", num);

                 return Some( WheelDirection::BACK);
             }
         }
         // 更新状态
         last_a_state = current_a_state;
         last_b_state = current_b_state;
     }

    None
}