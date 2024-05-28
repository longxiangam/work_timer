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
    current_count:u32,
    need_render:bool,
    choose_index:u32,
    running:bool,
    loading:bool,
    error:Option<String>
}

impl TimerPage {

    fn increase(&mut self) {
        if self.choose_index < 500 {
            self.choose_index += 1;
            self.need_render = true;
        }
    }

    fn decrease(&mut self) {
        if self.choose_index > 0 {
            self.choose_index -= 1;
            self.need_render = true;
        }
    }
    fn back(&mut self){
        self.running = false;
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
            running:true,
            choose_index: 0,
            loading: false,
            error: None,
        }
    }
    async fn bind_event(&mut self) {
        event::clear().await;

        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  move |ptr|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                println!("count_down_page:{}",mut_ref.choose_index );
            });
        }).await;
        event::on_target(EventType::KeyShort(1),Self::mut_to_ptr(self),  move |ptr|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.increase();
                println!("count_down_page:{}",mut_ref.choose_index );
            });
        }).await;
        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |ptr|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.back();
                println!("count_down_page:{}",mut_ref.choose_index );
            });
        }).await;
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);
                let style = MonoTextStyleBuilder::new()
                    .font(&embedded_graphics::mono_font::iso_8859_16::FONT_9X18)
                    .text_color(TwoBitColor::Black)
                    .background_color(TwoBitColor::White)
                    .build();

                let style =
                    U8g2TextStyle::new(fonts::u8g2_font_wqy12_t_gb2312b, TwoBitColor::Black);

                let display_area = display.bounding_box();

                let position = display_area.center();
                if self.loading {
                    let _ = Text::new("加载中。。。", Point::new(0,50), style.clone()).draw(display);
                }else{

                    if let Some(e) =  &self.error {
                        let _ = Text::new(format!("加载失败,{}",e).as_str(), Point::new(0,50), style.clone()).draw(display);
                    }else{
                        if *CLOCK_SYNC_SUCCESS.lock().await {
                            if let Some(clock) = get_clock() {
                                let local = clock.local().await;
                                let hour = local.hour();
                                let minute = local.minute();
                                let second = local.second();


                                let str = format_args!("{:02}:{:02}:{:02}",hour,minute,second).to_string();
                                Self::draw_clock(display,str.as_str());
                                let time = clock.get_date_str().await;
                                let _ = Text::new(time.as_str(), Point::new(0, 12), style.clone()).draw(display);
                            }
                        }else{
                            let _ = Text::new("同步时间...", Point::new(0,50), style.clone()).draw(display);
                        }
                    }

                }

                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;

            }
        }
    }

    async fn run(&mut self,spawner: Spawner) {
        self.running = true;
        loop {

            if !self.running {
                break;
            }
            self.need_render = true;
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