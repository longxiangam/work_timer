use alloc::boxed::Box;
use alloc::format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
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
use crate::pages::count_down_page::{ CountDownPage};
use crate::pages::{MAIN_PAGE, Page};


pub static MAIN_PAGE_CHANNEL: Channel<CriticalSectionRawMutex,MainPageInfo, 64> = Channel::new();

pub struct MainPageInfo{
    pub count:i32
}

//每个page 包含状态与绘制与逻辑处理
pub struct MainPage{
    current_page:u32,
}

impl MainPage {

    pub async fn init(){
        MAIN_PAGE.lock().await.get_mut().replace(MainPage::new());
        Self::bind_event().await;
    }

    pub async fn get_mut() -> Option<&'static mut MainPage> {
        unsafe {
            // 一个 u64 值，假设它是一个有效的指针地址

            // 将 u64 转换为指针类型
            let ptr: *mut MainPage =  MAIN_PAGE.lock().await.borrow_mut().as_mut().unwrap()  as *mut MainPage;
            return Some(&mut *ptr);
        }
    }
    async fn bind_event(){
        event::clear().await;
        event::on(EventType::KeyShort(1),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().current_page += 1;
                println!("current_page:{}",Self::get_mut().await.unwrap().current_page );
            });
        }).await;
    }
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
            draw_text_2(display_mut().unwrap(),format!("render:{}", self.current_page).as_str(),10,50,TwoBitColor::Black);
            RENDER_CHANNEL.send(RenderInfo{time:0}).await;
            println!("has display:{}",self.current_page);
        }else{
            println!("no display");
        }
    }

    async fn run(&mut self){

        if self.current_page == 0 {
            //监听事件
            println!("main_pages");
            self.render().await;
        }else if self.current_page == 1{
          /*  let mut count_down = CountDownPage::new();
            count_down.run().await;
            self.current_page = 0;*/
        }


    }




}


