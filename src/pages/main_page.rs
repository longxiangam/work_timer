use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::fmt::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::convert::Infallible;
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
use crate::pages::count_down_page::{ CountDownPage};
use crate::pages::{ Page};
use crate::widgets::list_widget::ListWidget;

static MAIN_PAGE:Mutex<CriticalSectionRawMutex,RefCell<Option<MainPage>> > = Mutex::new(RefCell::new(None));
pub static MAIN_PAGE_CHANNEL: Channel<CriticalSectionRawMutex,MainPageInfo, 64> = Channel::new();

pub struct MainPageInfo{
    pub count:i32
}

struct  MenuItem{
    key:String,
    title:String,
}

///每个page 包含状态与绘制与逻辑处理
///状态通过事件改变，并触发绘制
pub struct MainPage{
    current_page:u32,
    choose_index:u32,
    is_long_start:bool,
    need_render:bool,
    menus:Option<Vec<MenuItem>>
}

impl MainPage {

    pub async fn init(spawner: Spawner){
        MAIN_PAGE.lock().await.get_mut().replace(MainPage::new());
        spawner.spawn(increase()).ok();
        spawner.spawn(decrease()).ok();
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
        event::on(EventType::KeyShort(1),  move ||  {
            println!("current_page:" );
            return Box::pin(async {
                Self::get_mut().await.unwrap().increase();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
        event::on(EventType::KeyLongStart(1),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                INCREASE_CHANNEL.send(true).await;
            });
        }).await;

        event::on(EventType::KeyLongEnd(1),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                INCREASE_CHANNEL.send(false).await;
            });
        }).await;
        event::on(EventType::KeyLongStart(2),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                DECREASE_CHANNEL.send(true).await;
            });
        }).await;

        event::on(EventType::KeyLongEnd(2),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                DECREASE_CHANNEL.send(false).await;
            });
        }).await;
        event::on(EventType::KeyShort(2),  ||  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().decrease();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;

        event::on(EventType::WheelFront,  ||  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().increase();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;

        event::on(EventType::WheelBack,  ||  {
            println!("current_page:" );
            return Box::pin( async {
                Self::get_mut().await.unwrap().decrease();
                println!("current_page:{}",Self::get_mut().await.unwrap().choose_index );
            });
        }).await;
    }


    fn increase(&mut self){
        if self.choose_index < self.menus.as_mut().unwrap().len() as u32 {
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
        self.current_page = 0;
        self.need_render = true;
        Self::bind_event().await;
    }
}
impl Page for  MainPage{

    fn new()->Self{


        let mut menus = vec![];
        for i in 0..20 {
            menus.push(MenuItem{
                title:format!("菜单项{}",i),
                key:i.to_string()
            });
        }
        Self{
            current_page:0,
            choose_index:0,
            is_long_start:false,
            need_render:true,
            menus:Some(menus)
        }
    }
    //通过具体的状态绘制
    async fn render(&mut self) {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                display.clear(TwoBitColor::White);
                let menus:Vec<&str> = self.menus.as_ref().unwrap().iter().map(|v|{ v.title.as_str() }).collect();


                let mut list_widget = ListWidget::new(Point::new(0, 0)
                                                      , TwoBitColor::Black
                                                      , TwoBitColor::White
                                                      , display.bounding_box().size
                                                      , menus
                );
                list_widget.choose(self.choose_index as usize);
                list_widget.draw(display);
                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
                println!("has display:{}", self.choose_index);


            } else {
                println!("no display");
            }
        }

    }

    async fn run(&mut self,spawner: Spawner){

        loop {
            if self.current_page == 0 {
                //监听事件
                self.render().await;
            } else if self.current_page == 1 {
                CountDownPage::init(spawner).await;
                CountDownPage::get_mut().await.unwrap().run(spawner).await;

                //切换到主页并绑定事件
                self.back().await;
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

