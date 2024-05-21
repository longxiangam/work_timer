use alloc::boxed::Box;
use core::cell::RefCell;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
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


static COUNT_DOWN_PAGE:Mutex<CriticalSectionRawMutex,RefCell<Option<CountDownPage>> > = Mutex::new(RefCell::new(None));

pub struct  CountDownPage{
    begin_count:u32,
    current_count:u32,
    need_render:bool,
    choose_index:u32,
}

impl CountDownPage {
    pub async fn init(spawner: Spawner) {
        COUNT_DOWN_PAGE.lock().await.get_mut().replace(CountDownPage::new());
        Self::bind_event().await;
    }
    pub async fn close(){
        event::clear().await;
        COUNT_DOWN_PAGE.lock().await.replace(None);
    }

    pub async fn get_mut() -> Option<&'static mut Self> {
        unsafe {
            // 一个 u64 值，假设它是一个有效的指针地址

            // 将 u64 转换为指针类型
            let ptr: *mut CountDownPage = COUNT_DOWN_PAGE.lock().await.borrow_mut().as_mut().unwrap() as *mut CountDownPage;
            return Some(&mut *ptr);
        }
    }
    async fn bind_event() {
        event::clear().await;
    }
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
}

impl Page for CountDownPage{
    fn new() -> Self {
        Self{
            begin_count:0,
            current_count:0,
            need_render:true,
            choose_index: 0,
        }
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {

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
        loop {

            //监听事件
            println!("main_pages");

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
#[embassy_executor::task]
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

