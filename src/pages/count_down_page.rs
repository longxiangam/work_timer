use alloc::boxed::Box;
use core::cell::RefCell;
use core::future::Future;
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
use embedded_graphics::prelude::Dimensions;
use embedded_graphics::text::Text;
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

pub struct  CountDownPage{
    begin_count:u32,
    current_count:u32,
    need_render:bool,
    choose_index:u32,
    running:bool,
}

impl CountDownPage {


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

    async fn request(&mut self){
        let stack = use_wifi().await;
        if let Ok(v) = stack {
            let mut request = RequestClient::new(v);
            let result = request.send_request("").await;
            match result {
                Ok(response) => {
                    println!("请求成功{}", core::str::from_utf8(& response.data[..response.length]).unwrap());
                }
                Err(e) => {
                    println!("请求失败{:?}",e);
                }
            }

            println!("get stack ok" );
        }else{
            println!("get stack err" );
        }
        println!("get stack" );
    }
}

impl Page for CountDownPage{
    fn new() -> Self {
        Self{
            begin_count:0,
            current_count:0,
            need_render:true,
            running:true,
            choose_index: 0,
        }
    }
    async fn bind_event(&mut self) {
        event::clear().await;

       /* let temp = move |aa| {
            return Box::pin(async move {
                let ptr = None;
                let mut_ref: &mut Self = Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.request().await;
                println!("count_down_page:{}", mut_ref.choose_index);
            });
        };
        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  temp).await;*/
        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  move |ptr|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.request().await;
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
                display.clear(TwoBitColor::White);
                let style = MonoTextStyleBuilder::new()
                    .font(&embedded_graphics::mono_font::iso_8859_16::FONT_9X18)
                    .text_color(TwoBitColor::Black)
                    .background_color(TwoBitColor::White)
                    .build();

                let style =
                    U8g2TextStyle::new(fonts::u8g2_font_wqy12_t_gb2312b, TwoBitColor::Black);

                let display_area = display.bounding_box();
                let row = LinearLayout::horizontal(
                    Chain::new(Text::new("时间:", Point::zero(), style.clone()))
                        .append(Text::new("10", Point::zero(), style.clone()))
                        .append(Text::new("分钟", Point::zero(), style.clone())),
                )
                    .with_alignment(vertical::Center)
                    .arrange();
                LinearLayout::vertical( Chain::new(row) )
                    .with_alignment(horizontal::Left)
                    .arrange()
                    .align_to(&display_area, horizontal::Left, vertical::Top)
                    .draw(display);

                Text::new(" 时间:  时间: ", Point::new(0,50), style.clone()).draw(display);



                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;

            }
        }
    }

    async fn run(&mut self,spawner: Spawner) {
        let mut last_secs = Instant::now().as_secs();
        self.running = true;
        loop {

            if !self.running {
                break;
            }

            let current_secs = Instant::now().as_secs();
            if current_secs != last_secs {
                self.increase();
                last_secs = current_secs;
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
