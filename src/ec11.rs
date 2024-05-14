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
use crate::ec11::BeginState::{BACK, FRONT, NO_STATE};

#[derive(Eq, PartialEq)]
enum BeginState{
    FRONT,
    BACK,
    NO_STATE,
}
const SAMPLE_TIMES:u32 = 10;
const JUDGE_TIMES:u32 = 8;

#[embassy_executor::task]
pub async fn task(mut a_point :Gpio1<Input<PullUp>>,mut b_point :Gpio0<Input<PullUp>>){
    // 初始化编码器状态

    let mut begin_state = BeginState::NO_STATE;
    let renderSender = display::RENDER_CHANNEL.sender();
    // 初始化编码器状态
    let mut last_a_state = a_point.is_low();
    let mut last_b_state = b_point.is_low();

    // 开始监听编码器状态变化
    let mut num:i32 = 0;
    loop {
        //select( a_point.wait_for_any_edge(),b_point.wait_for_any_edge()).await;
        a_point.wait_for_any_edge().await;

        println!("ticks:{}",1111111111111);

        let mut current_a_state = a_point.is_low();
        let mut current_b_state = b_point.is_low();

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
        println!("a_is_low_times:{}",a_is_low_times);
        println!("b_is_low_times:{}",b_is_low_times);
        let mut a_is_down = false;
        let mut b_is_down = false;
        if(a_is_low_times > JUDGE_TIMES){
            println!("下降沿");
            a_is_down = true;
        }else if a_is_low_times < SAMPLE_TIMES - JUDGE_TIMES {
            println!("上升沿");
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

        let ticks = Instant::now().as_millis();
        println!("ticks:{}",ticks);

        //下降沿开始
        if(a_is_down){
            if(b_is_down){
                begin_state = FRONT;
                println!("开始正转");
                continue;
            }else if(!b_is_down){
                begin_state =  BACK;
                println!("开始反转");
                continue;
            }
            begin_state = NO_STATE;

        }else{
            //上升沿判断结束
            if(!b_is_down){
                if begin_state == FRONT  {
                    println!("正转");
                    num +=10;
                    renderSender.send(RenderInfo{num}).await;
                }
            }else if(b_is_down){
                if begin_state == BACK {
                    println!("反转");
                    num -=10;
                    renderSender.send(RenderInfo{num}).await;
                }
            }
            begin_state = NO_STATE;
        }

        println!("num:{}",num);

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
       /* let mut current_a_state = a_point.is_low();
        let mut current_b_state = b_point.is_low();
        if current_a_state != last_a_state || current_b_state != last_b_state {
            if last_a_state.unwrap() && last_b_state.unwrap() {
                if current_a_state.unwrap() && !current_b_state.unwrap() {
                    // 正转
                    num = num + 1;
                    println!("正转");
                    println!("num :{}", num);
                    renderSender.send(RenderInfo{num}).await;
                } else if !current_a_state.unwrap() && current_b_state.unwrap() {
                    // 反转
                    num = num - 1;
                    println!("反转");
                    println!("num :{}", num);
                    renderSender.send(RenderInfo{num}).await;
                }
            }
            // 更新状态
            last_a_state = current_a_state;
            last_b_state = current_b_state;
        }*/

    }

}



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