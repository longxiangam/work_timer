use alloc::boxed::Box;
use alloc::fmt::format;
use heapless::String;
use heapless::Vec;
use core::cell::RefCell;
use core::convert::Infallible;
use core::str::FromStr;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select, Select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, ReceiveFuture};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Dimensions;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, Size};
use embedded_graphics::primitives::{Line, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, StyledDrawable};
use embedded_graphics::text::Text;
use embedded_io_async::Write;
use embedded_layout::align::{Align, horizontal, vertical};
use embedded_layout::layout::linear::{FixedMargin, LinearLayout};
use embedded_layout::layout::linear::spacing::DistributeFill;
use embedded_layout::object_chain::Chain;
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use static_cell::make_static;
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

use crate::{display, event};
use crate::display::{display_mut, draw_text_2, RENDER_CHANNEL, RenderInfo};
use crate::event::EventType;
use crate::pages::clock_page::{ClockPage};
use crate::pages::{MenuItem, Page, PageEnum};
use crate::pages::calender_page::CalenderPage;
use crate::pages::games_page::GamesPage;
use crate::pages::PageEnum::{ECalenderPage, EChip8Page, EClockPage, ETimerPage, EWeatherPage};
use crate::pages::timer_page::TimerPage;
use crate::pages::weather_page::WeatherPage;
use crate::widgets::list_widget::ListWidget;

static MAIN_PAGE:Mutex<CriticalSectionRawMutex,RefCell<Option<MainPage>> > = Mutex::new(RefCell::new(None));
pub static MAIN_PAGE_CHANNEL: Channel<CriticalSectionRawMutex,MainPageInfo, 64> = Channel::new();

pub struct MainPageInfo{
    pub count:i32
}



///每个page 包含状态与绘制与逻辑处理
///状态通过事件改变，并触发绘制
pub struct MainPage{
    current_page:Option<u32>,
    choose_index:u32,
    is_long_start:bool,
    need_render:bool,
    menus:Option<Vec<MenuItem,20>>
}

impl MainPage {

    pub async fn init(spawner: Spawner){
        MAIN_PAGE.lock().await.get_mut().replace(MainPage::new());
        spawner.spawn(increase()).ok();
        spawner.spawn(decrease()).ok();
        Self::bind_event(MAIN_PAGE.lock().await.get_mut().as_mut().unwrap()).await;
    }

    pub async fn get_mut() -> Option<&'static mut MainPage> {
        unsafe {
            // 一个 u64 值，假设它是一个有效的指针地址

            // 将 u64 转换为指针类型
            let ptr: *mut MainPage =  MAIN_PAGE.lock().await.borrow_mut().as_mut().unwrap()  as *mut MainPage;
            return Some(&mut *ptr);
        }
    }


    fn increase(&mut self){
        if self.choose_index < (self.menus.as_mut().unwrap().len() - 1) as u32 {
            self.choose_index += 1;
            self.need_render = true;
        }
    }

    fn decrease(&mut self){
        if self.choose_index > 0 {
            self.choose_index -= 1;
            self.need_render = true;
        }
    }

    async fn back(&mut self){
        self.current_page = None;
        self.need_render = true;
        Self::bind_event(self).await;
    }
}
impl Page for  MainPage{

    fn new()->Self{

        let mut menus = Vec::new();
        menus.push(MenuItem::new(String::<20>::from_str("时钟").unwrap(), EClockPage));
        menus.push(MenuItem::new(String::<20>::from_str("定时器").unwrap(), ETimerPage));
        menus.push(MenuItem::new(String::<20>::from_str("天气").unwrap(), EWeatherPage));
        menus.push(MenuItem::new(String::<20>::from_str("日历").unwrap(), ECalenderPage));
        menus.push(MenuItem::new(String::<20>::from_str("游戏").unwrap(), EChip8Page));

       /*     MenuItem::new(String::from_str("时钟"), EClockPage),
            MenuItem::new("定时器".to_string(), ETimerPage),
            MenuItem::new("天气".to_string(), EWeatherPage),
            MenuItem::new("日历".to_string(), ECalenderPage),
            MenuItem::new("游戏".to_string(), EChip8Page),
        */
        Self{
            current_page:None,
            choose_index:0,
            is_long_start:false,
            need_render:true,
            menus:Some(menus)
        }
    }
    async fn bind_event(&mut self){
        event::clear().await;
        event::on(EventType::KeyShort(1),  move |ptr|  {
            println!("current_page:" );
            return Box::pin(async {
                Self::get_mut().await.unwrap().increase();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
        event::on(EventType::KeyLongStart(1),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                INCREASE_CHANNEL.send(true).await;
            });
        }).await;

        event::on(EventType::KeyLongEnd(1),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                INCREASE_CHANNEL.send(false).await;
            });
        }).await;
        event::on(EventType::KeyLongStart(2),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                DECREASE_CHANNEL.send(true).await;
            });
        }).await;

        event::on(EventType::KeyLongEnd(2),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                DECREASE_CHANNEL.send(false).await;
            });
        }).await;
        event::on(EventType::KeyShort(2),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().decrease();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
        event::on(EventType::KeyShort(5),  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                let mut_ref = Self::get_mut().await.unwrap();
                mut_ref.current_page = Some( mut_ref.choose_index);
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
        event::on(EventType::WheelFront,  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().increase();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;

        event::on(EventType::WheelBack,  |ptr|  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().decrease();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
    }



    //通过具体的状态绘制
    async fn render(&mut self) {
        if self.need_render {

            if let Some(display) = display_mut() {
                self.need_render = false;

                let _ = display.clear(TwoBitColor::White);
                let menus:Vec<&str,20> = self.menus.as_ref().unwrap().iter().map(|v|{ v.title.as_str() }).collect();


                let mut list_widget = ListWidget::new(Point::new(0, 0)
                                                      , TwoBitColor::Black
                                                      , TwoBitColor::White
                                                      , display.bounding_box().size
                                                      , menus
                );
                list_widget.choose(self.choose_index as usize);
                let _ = list_widget.draw(display);
                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
                println!("has display:{}", self.choose_index);


            } else {
                println!("no display");
            }
        }

    }

    async fn run(&mut self,spawner: Spawner){

        loop {
            if  None == self.current_page {
                self.render().await;
                Timer::after(Duration::from_millis(50)).await;
                continue;
            }
            let current_page = self.current_page.unwrap();
            let menuItem = &self.menus.as_mut().unwrap()[current_page as usize];
            match menuItem.page_enum {
                PageEnum::EMainPage => {

                }
                EClockPage => {
                    let mut clock_page = ClockPage::new();
                    clock_page.bind_event().await;
                    clock_page.run(spawner).await;

                    //切换到主页并绑定事件
                    self.back().await;
                }
                ETimerPage => {
                    let mut timer_page = TimerPage::new();
                    timer_page.bind_event().await;
                    timer_page.run(spawner).await;
                    self.back().await;
                }
                EWeatherPage => {
                    let mut clock_page = WeatherPage::new();
                    clock_page.bind_event().await;
                    clock_page.run(spawner).await;
                    self.back().await;
                }
                ECalenderPage => {
                    let mut calender_page = CalenderPage::new();
                    calender_page.bind_event().await;
                    calender_page.run(spawner).await;
                    self.back().await;
                }
                EChip8Page => {
                    let mut games_page = GamesPage::new();
                    games_page.bind_event().await;
                    games_page.run(spawner).await;

                    //切换到主页并绑定事件
                    self.back().await;
                }


            }

            Timer::after(Duration::from_millis(50)).await;
        }
    }

}


static INCREASE_CHANNEL:Channel<CriticalSectionRawMutex,bool, 2> = Channel::new();
static DECREASE_CHANNEL:Channel<CriticalSectionRawMutex,bool, 2> = Channel::new();
#[embassy_executor::task]
async fn increase(){
    loop {
        INCREASE_CHANNEL.receive().await;
        loop {
            let a = Timer::after(Duration::from_millis(100));
            let b = INCREASE_CHANNEL.receive();
            match select(a,b).await {
                Either::First(_) => {
                    MainPage::get_mut().await.unwrap().increase();
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
                    MainPage::get_mut().await.unwrap().decrease();
                }
                Either::Second(_) => {
                    break;
                }
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

