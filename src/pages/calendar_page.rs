use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::{DrawTarget, OriginDimensions, Point, Size};
use embedded_graphics::text::{Baseline, Text, TextStyle, TextStyleBuilder};
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use crate::pages::Page;
use time::{Date, OffsetDateTime, Month};
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;
use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::event;
use crate::event::EventType;
use crate::widgets::calendar::Calendar;
use crate::worldtime::{CLOCK_SYNC_SUCCESS, get_clock};

pub struct CalendarPage {
    running:bool,
    need_render:bool,
    calendar:Option<Calendar<TwoBitColor>>
}

impl CalendarPage {

    async fn back(&mut self){
        self.running = false;
    }
}

impl Page for CalendarPage {
     fn new() -> Self {
        let mut calendar = None;
        Self{
            running: false,
            need_render: false,
            calendar
        }
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);

                if let Some(ref mut calendar) = self.calendar {
                    calendar.position = Point::new(0,0);
                    calendar.size = Size::new(display.size().width /2,display.size().height);
                    calendar.draw(display);
                }

            }

            RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
        }
    }

    async fn run(&mut self, spawner: Spawner) {
        self.running = true;

        loop {
            if !self.running {
                break;
            }
            if self.calendar == None {
                if *CLOCK_SYNC_SUCCESS.lock().await {
                    if let Some(clock) = get_clock() {
                        let local = clock.local().await;
                        let year = local.year();
                        let month = local.month();
                        let today = local.date();
                        self.calendar = Some(Calendar::new(Point::default(),Size::default(),year,month,today,TwoBitColor::Black,TwoBitColor::White));
                    }
                }
            }
            self.need_render = true;
            self.render().await;
            Timer::after(Duration::from_millis(50)).await;
        }
    }

    async fn bind_event(&mut self) {
        event::clear().await;
        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |ptr|  {
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.back().await;
            });
        }).await;
    }
}
