
use embassy_futures::select::{Either, select};
use embassy_time::{Instant};

use embedded_hal_async::digital::Wait;
use esp_println::{ println};
use hal::gpio::{ Gpio0, Gpio1, Gpio13, Input,  PullUp};
use hal::prelude::_embedded_hal_digital_v2_InputPin;


use crate::ec11::WheelDirection::{Back, Front, NoState};
use crate::event::{EventType, key_detection, toggle_event};

#[derive(Eq, PartialEq)]
enum WheelDirection{
    Front,
    Back,
    NoState,
}
const SAMPLE_TIMES:u32 = 10;
const JUDGE_TIMES:u32 = 8;

#[embassy_executor::task]
pub async fn task(mut a_point :Gpio1<Input<PullUp>>,mut b_point :Gpio0<Input<PullUp>>,mut push_key:Gpio13<Input<PullUp>>){
    // 初始化编码器状态

    let mut begin_state = NoState;

    // 开始监听编码器状态变化
    loop {

        let a_edge =  a_point.wait_for_any_edge();
        let key_edge =push_key.wait_for_falling_edge();

        match  select(a_edge,key_edge).await {
            Either::First(_) => {

                let mut a_is_low_times = 0;
                let mut b_is_low_times = 0;
                for _i in 0..SAMPLE_TIMES {
                    if a_point.is_low().unwrap() {
                        a_is_low_times += 1;
                    }
                    if b_point.is_low().unwrap() {
                        b_is_low_times += 1;
                    }
                }

                let mut a_is_down = false;
                let mut b_is_down = false;
                if a_is_low_times > JUDGE_TIMES {
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
                if a_is_down {
                    if b_is_down {
                        begin_state = Front;
                        continue;
                    }else if !b_is_down {
                        begin_state = Back;
                        continue;
                    }
                    begin_state = NoState;

                }else{
                    //上升沿判断结束
                    if !b_is_down {
                        if begin_state == Front {
                            toggle_event(EventType::WheelFront,Instant::now().as_millis()).await;
                        }
                    }else if b_is_down {
                        if begin_state == Back {
                            toggle_event(EventType::WheelBack,Instant::now().as_millis()).await;
                        }
                    }
                    begin_state = NoState;
                }


            }
            Either::Second(_) => {
                key_detection::<_,5>(&mut push_key).await;
            }
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
                 return Some( WheelDirection::Front);
             } else if !current_a_state.unwrap() && current_b_state.unwrap() {
                 // 反转
                 num = num - 1;
                 println!("反转");
                 println!("num :{}", num);

                 return Some( WheelDirection::Back);
             }
         }
         // 更新状态
         last_a_state = current_a_state;
         last_b_state = current_b_state;
     }

    None
}