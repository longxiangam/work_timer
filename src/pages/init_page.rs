use alloc::string::ToString;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::prelude::{DrawTarget, OriginDimensions};
use embedded_graphics::primitives::Rectangle;
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use lcd_drivers::uc1638::prelude::Display2in7;
use u8g2_fonts::FontRenderer;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};
use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::pages::Page;
use crate::widgets::calendar::Calendar;
use crate::widgets::clock_widget::ClockWidget;
use crate::worldtime::{CLOCK_SYNC_TIME_SECOND, get_clock};
use u8g2_fonts::fonts;
use crate::wifi::use_wifi;


pub struct InitPage{
    top:i32,
}

impl InitPage{
    pub async fn append_log(&mut self, str:&str){
        loop {
            if let Some(display) = display_mut() {
                println!("append_log display");
                if (self.top == 0) {
                    let _ = display.clear(TwoBitColor::White);
                }
                let rect = Rectangle::new(Point::new(0, self.top)
                                          , Size::new(display.size().width, 20));
                let font: FontRenderer = FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>();
                let _ = font.render_aligned(
                    str,
                    rect.center(),
                    VerticalPosition::Center,
                    HorizontalAlignment::Center,
                    FontColor::Transparent(TwoBitColor::Black),
                    display,
                );
                self.top += 20;
                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
                break;
            }
            Timer::after_millis(10).await;
        }
    }
}

impl Page for InitPage{
    fn new() -> Self {
        Self{
            top: 0,
        }
    }

    async fn render(&mut self) {

    }



    async fn run(&mut self, spawner: Spawner) {

       /* self.append_log("正在连接wifi").await;

        //网络
        loop {

            let stack = use_wifi().await;
            if let Ok(v) = stack {

                loop {
                    if v.is_link_up() {
                        break;
                    }

                    Timer::after_millis(50);
                }

                self.append_log("已连接wifi,正在获取ip").await;

                loop {
                    if let Some(config) = v.config_v4() {
                        unsafe {
                            crate::wifi::IP_ADDRESS = config.address.address().to_string().parse().unwrap();
                        }
                        break;
                    }
                    Timer::after_millis(50);
                }

                self.append_log("已获取ip").await;
                break;
            }else {
                Timer::after_millis(50);
            }
        }

        //时间

        self.append_log("正在同步时间").await;

        loop {
            if let Some(clock) =  get_clock(){
                self.append_log("已成功同步时间").await;
                println!("Current_time: {}", clock.get_date_str().await);
                break;
            }
            Timer::after_millis(50).await;
        }

        self.append_log("进入系统").await;*/
    }

    async fn bind_event(&mut self) {
        todo!()
    }
}