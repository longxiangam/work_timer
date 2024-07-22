
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Instant};

use embedded_hal_async::digital::Wait;
use esp_println::{ println};
use hal::gpio::{Gpio0, Gpio1, Gpio13, Gpio5, Input };


use crate::ec11::WheelDirection::{Back, Front, NoState};
use crate::event::{ec11_toggle_event, EventType, key_detection, toggle_event};

#[derive(Eq, PartialEq,Debug,Copy, Clone)]
enum WheelDirection{
    Front,
    Back,
    NoState,
}
//转动时判断方向是否一致，
//一致则判断last_time是否过久，过久则重记时间
//方向一致且时间不过久则步长加1 ，更新 last_time ,并用 步数 / 时间得到速度，
#[derive(Eq, PartialEq,Debug,Copy, Clone)]
pub struct RotateState {
    pub begin_timestamp: u64,
    pub last_timestamp: u64,
    pub wheel_direction: WheelDirection,
    pub steps: u32,
}
impl RotateState {
    const fn new() -> Self {
        RotateState {
            begin_timestamp: 0,
            last_timestamp: 0,
            wheel_direction: WheelDirection::NoState,
            steps: 0,
        }
    }



    fn do_step(&mut self,wheel_direction: WheelDirection){
        const SPEED_DELAY:u64= 300;
        let ms = Instant::now().as_millis();
        if self.wheel_direction == wheel_direction {
            if ms - self.last_timestamp  < SPEED_DELAY{
                self.last_timestamp = ms;
                self.steps += 1;
                return;
            }
        }
        self.wheel_direction = wheel_direction;
        self.begin_timestamp = ms;
        self.last_timestamp = ms;
        self.steps = 1;

    }

    pub fn speed(&self)->f32{

        if self.steps > 3 {
            let speed = self.steps as f32 / (self.last_timestamp - self.begin_timestamp) as f32 * 1000.0;
            println!("begin_time:{:?}",self.begin_timestamp);
            println!("last_time:{:?}",self.last_timestamp);
            println!("steps:{:?}",self.steps);
            println!("speed:{:?}",speed);
            return speed;
        }else{
            return self.steps as f32;
        }

    }
}

static ROTATE_STATE:Mutex<CriticalSectionRawMutex,RotateState> = Mutex::new(RotateState::new());



const SAMPLE_TIMES:u32 = 10;
const JUDGE_TIMES:u32 = 8;


#[embassy_executor::task]
pub async fn task(mut a_point :Input<'_,Gpio1>,mut b_point :Input<'_,Gpio0>,mut push_key:Input<'_,Gpio5>){
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
                    if a_point.is_low() {
                        a_is_low_times += 1;
                    }
                    if b_point.is_low() {
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
                            ROTATE_STATE.lock().await.do_step(Front);
                            ec11_toggle_event(EventType::WheelFront, ROTATE_STATE.lock().await.clone()).await;
                        }
                    }else if b_is_down {
                        if begin_state == Back {
                            let speed = ROTATE_STATE.lock().await.do_step(Back);
                            ec11_toggle_event(EventType::WheelBack,ROTATE_STATE.lock().await.clone()).await;
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


fn detection(mut a_point :Input<Gpio1>, mut b_point :Input<Gpio0>) -> Option<WheelDirection> {
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
         if last_a_state && last_b_state {
             if current_a_state && !current_b_state {
                 // 正转
                 num = num + 1;
                 println!("正转");
                 println!("num :{}", num);
                 return Some( WheelDirection::Front);
             } else if !current_a_state && current_b_state {
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