use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::fmt::Debug;
use core::future::Future;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use embedded_layout::align::{Align, horizontal, vertical};
use embedded_layout::layout::linear::LinearLayout;
use embedded_layout::object_chain::Chain;
use esp_println::println;
use futures::FutureExt;
use lcd_drivers::color::TwoBitColor;
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::ec11::RotateState;
use crate::event;
use crate::event::EventType;
use crate::pages::{ Page};
use crate::pages::main_page::MainPage;
use crate::request::{RequestClient, ResponseData};
use crate::sound::{player_buzzer, SoundType, stop_buzzer};
use crate::wifi::use_wifi;
use crate::worldtime::{CLOCK_SYNC_TIME_SECOND, get_clock};

pub struct TimerPage {
    begin_count:i32,
    need_render:bool,
    current_count:i32,
    starting:bool,
    finished:bool,
    running:bool,
    loading:bool,
    error:Option<String>
}

impl TimerPage {

    fn increase(&mut self,speed:f32) {
        self.need_render = true;
        if !self.starting {
            if self.current_count < 3600 * 2 {
                self.current_count += speed as i32;
            }else{
                self.current_count = 3600 * 2;
            }
        }
    }

    fn decrease(&mut self,speed:f32) {
        self.need_render = true;
        if !self.starting {
            if self.current_count > 0 {
                self.current_count -=  speed as i32;
            }
            else {
                self.current_count = 0;
            }
        }
    }

    fn speed( rotate_state:RotateState)->f32{
        let mut speed = 10.0;
        let step_speed = rotate_state.steps  as f32/ 2.0 * 10.0;
        let time_speed = rotate_state.speed();
        println!("state:{:?}",rotate_state);
        println!("step_speed:{}",step_speed);
        println!("time_speed:{}",time_speed);
        println!("steps:{}",rotate_state.steps);
        if rotate_state.steps > 10{
            speed = 120.0;
        } else if rotate_state.steps > 5 {
            speed = 60.0;
        }


        println!("11 speed:{}",speed);
        speed
    }
    async fn step(&mut self){
        if self.begin_count == 0 {
            //正
            self.current_count +=1;

        }else{
            //倒
            if self.current_count > 0{
                self.finished = false;
                self.current_count -=1;
            }
            if self.current_count == 0 {
                self.finished = true;
                println!("player");
                player_buzzer(SoundType::Music(1)).await;
            }
        }

        self.need_render = true;
    }

    async fn back(&mut self){
        stop_buzzer().await;
        self.running = false;
    }

    async fn toggle_starting(&mut self){

        self.need_render = true;

        if self.finished {
            self.starting = false;
            self.finished = false;
            stop_buzzer().await;
            return;
        }

        if self.starting {
            self.starting = false;
        }else{
            self.starting = true;
            self.begin_count = self.current_count;
        }
    }

    fn draw_clock<D>(display: &mut D, time: &str) -> Result<(), D::Error>
        where
            D: DrawTarget<Color = TwoBitColor>,
    {
        let character_style = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(30, 60))
            .segment_width(5)
            .segment_color(TwoBitColor::Black)
            .build();

        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Middle)
            .build();

        Text::with_text_style(
            &time,
            display.bounding_box().center(),
            character_style,
            text_style,
        )
            .draw(display)?;

        Ok(())
    }

}

impl Page for TimerPage {
    fn new() -> Self {
        Self{
            begin_count:0,
            current_count:0,
            need_render:true,
            starting:false,
            finished: false,
            running:true,
            loading: false,
            error: None,
        }
    }
    async fn bind_event(&mut self) {
        event::clear().await;
        event::on_target(EventType::KeyShort(3),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.toggle_starting().await;
            });
        }).await;
        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.decrease(5.0);
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;
        event::on_target(EventType::KeyLongIng(2),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.decrease(5.0);
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;
        event::on_target(EventType::KeyShort(1),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.increase(5.0);
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;
        event::on_target(EventType::KeyLongIng(1),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.increase(5.0);
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;


        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.back().await;

                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;


        event::on_target(EventType::WheelFront,Self::mut_to_ptr(self),  |info|  {
            println!("current_page:" );
            return Box::pin( async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.increase(Self::speed(info.rotate_state.unwrap()));
            });
        }).await;

        event::on_target(EventType::WheelBack,Self::mut_to_ptr(self),  |info|  {
            println!("current_page:" );
            return Box::pin( async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.decrease(Self::speed(info.rotate_state.unwrap()));
            });
        }).await;
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);

                if self.finished {
                    //闪烁一下
                    if Instant::now().as_secs() % 2 == 0 {
                        RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
                        return;
                    }
                }
                let second =   self.current_count%60;
                let minute = self.current_count / 60 % 60;
                let hour = self.current_count / 3600;

                let time = format!("{:02}:{:02}:{:02}",hour,minute,second);

                Self::draw_clock(display,time.as_str());

                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
            }
        }
    }

    async fn run(&mut self,spawner: Spawner) {
        self.running = true;
        let mut last_time = 0 ;
        loop {
            if !self.running {
                break;
            }

            if self.starting && !self.finished {
                if last_time == 0 {
                    last_time = Instant::now().as_secs();
                }
                if Instant::now().as_secs() > last_time {
                    last_time = Instant::now().as_secs();
                    self.step().await;
                }
            }
            if self.finished {
                self.need_render = true;
            }

            self.render().await;
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}



static INCREASE_CHANNEL:Channel<CriticalSectionRawMutex,bool, 2> = Channel::new();
static DECREASE_CHANNEL:Channel<CriticalSectionRawMutex,bool, 2> = Channel::new();
/*#[embassy_executor::task]
async fn increase(){
    loop {
        INCREASE_CHANNEL.receive().await;
        loop {
            let a = Timer::after(Duration::from_millis(100));
            let b = INCREASE_CHANNEL.receive();
            match select(a,b).await {
                Either::First(_) => {
                    CountDownPage::get_mut().await.unwrap().increase();
                }
                Either::Second(_) => {
                    break;
                }
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}


#[embassy_executor::task]
async fn decrease(){
    loop {
        DECREASE_CHANNEL.receive().await;
        loop {
            let a = Timer::after(Duration::from_millis(100));
            let b = DECREASE_CHANNEL.receive();
            match select(a,b).await {
                Either::First(_) => {
                    CountDownPage::get_mut().await.unwrap().decrease();
                }
                Either::Second(_) => {
                    break;
                }
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}
*/
