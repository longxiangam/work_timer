use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::cell::RefCell;
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
use lcd_drivers::color::TwoBitColor;
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::event;
use crate::event::EventType;
use crate::pages::{ Page};
use crate::pages::main_page::MainPage;
use crate::request::{RequestClient, ResponseData};
use crate::wifi::use_wifi;
use crate::worldtime::{CLOCK_SYNC_SUCCESS, get_clock};

pub struct TimerPage {
    begin_count:u32,
    need_render:bool,
    current_count:u32,
    starting:bool,
    finished:bool,
    running:bool,
    loading:bool,
    error:Option<String>
}

impl TimerPage {

    fn increase(&mut self) {
        if !self.starting {
            if self.current_count < 3600 * 2 {
                self.current_count += 10;
                self.need_render = true;
            }else{
                self.current_count = 3600 * 2;
                self.need_render = true;
            }
        }
    }

    fn decrease(&mut self) {
        if !self.starting {
            if self.current_count > 10 {
                self.current_count -= 10;
                self.need_render = true;
            }else {
                self.current_count = 0;
                self.need_render = true;
            }
        }
    }

    fn step(&mut self){
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
            }
        }

        self.need_render = true;
    }

    fn back(&mut self){
        self.running = false;
    }

    fn toggle_starting(&mut self){
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

        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.decrease();
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;
        event::on_target(EventType::KeyShort(1),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.increase();
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;

        event::on_target(EventType::KeyShort(4),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.back();
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;

        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.toggle_starting();
                println!("count_down_page:{}",mut_ref.current_count );
            });
        }).await;


        event::on_target(EventType::WheelFront,Self::mut_to_ptr(self),  |info|  {
            println!("current_page:" );
            return Box::pin( async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.increase();
            });
        }).await;

        event::on_target(EventType::WheelBack,Self::mut_to_ptr(self),  |info|  {
            println!("current_page:" );
            return Box::pin( async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.decrease();
            });
        }).await;
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);

                let second =  self.current_count%60;
                let minute = self.current_count / 60 % 60;
                let hour = self.current_count / 3600;

                let time = format!("{hour}:{minute}:{second}");

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

            if self.starting {
                if last_time == 0 {
                    last_time = Instant::now().as_secs();
                }
                if Instant::now().as_secs() > last_time {
                    last_time = Instant::now().as_secs();
                    self.step();
                }
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
