use alloc::format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_io_async::Write;
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use static_cell::make_static;
use crate::{display, event};
use crate::display::{display_mut, draw_text_2, RENDER_CHANNEL, RenderInfo};
use crate::event::EventType;
use crate::pages::count_down_page::CountDown;
use crate::pages::Page;


pub static MAIN_PAGE_CHANNEL: Channel<CriticalSectionRawMutex,MainPageInfo, 64> = Channel::new();

pub struct MainPageInfo{
    pub count:i32
}

//每个page 包含状态与绘制与逻辑处理
pub struct MainPage{
    current_page:u32,
}

impl Page for  MainPage{
    fn new()->Self{
        Self{
            current_page:0,
        }
    }
    //通过具体的状态绘制
    async fn render(&self) {
        if let Some(display) = display_mut() {
            display_mut().unwrap().fill_solid(&Rectangle::new(Point::new(10,50),Size::new(100,40)),TwoBitColor::White);
            draw_text_2(display_mut().unwrap(),format!("render:{}", 0).as_str(),10,50,TwoBitColor::Black);
            RENDER_CHANNEL.send(RenderInfo{time:0}).await;
            println!("has display");
        }else{
            println!("no display");
        }
    }

    async fn run(&mut self){

        if self.current_page == 0 {
            //监听事件
            event::clear().await;
            event::on(EventType::KeyShort(1), || {
                //self.current_page = 1;
            }).await;


            self.render().await;
        }else if self.current_page == 1{
            let mut count_down = CountDown::new();
            count_down.run().await;
            self.current_page = 0;
        }

    }

}





#[embassy_executor::task]
pub async fn main_task(){
    let mut main_page = make_static!(MainPage::new());
    loop {
        //main_page.run().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}
