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

//用于调试显示
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

    }

    async fn bind_event(&mut self) {
        todo!()
    }
}