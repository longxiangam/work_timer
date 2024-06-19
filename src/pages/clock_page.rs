use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};

use core::fmt::Debug;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_time::{Duration,  Timer};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::{Dimensions, Size};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use esp_println::println;
use lcd_drivers::color::TwoBitColor;


use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::event;
use crate::event::EventType;
use crate::model::seniverse::{DailyResult, form_json};

use crate::pages::{ Page};
use crate::pages::main_page::MainPage;
use crate::request::{RequestClient, ResponseData};
use crate::wifi::{finish_wifi, use_wifi};
use crate::worldtime::{get_clock, sync_time_success};

pub struct ClockPage {
    begin_count:u32,
    current_count:u32,
    need_render:bool,
    choose_index:u32,
    running:bool,
    loading:bool,
    error:Option<String>,
}

impl ClockPage {


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
        self.loading = true;
        self.error = None;
        let stack = use_wifi().await;
        if let Ok(v) = stack {
            println!("请求 stack 成功");
            let mut request = RequestClient::new(v).await;
            println!("开始请求成功");
            //let result = request.send_request("https://worldtimeapi.org/api/timezone/Europe/Copenhagen.txt").await;
            let result = request.send_request("http://api.seniverse.com/v3/weather/daily.json?key=SvRIiZPU5oGiqcHc1&location=beijing&language=en&unit=c&start=0&days=5").await;
            match result {
                Ok(response) => {
                    finish_wifi().await;
                    self.loading = false;
                    self.error = None;
                    let daily_result = form_json(&response.data[..response.length]);

                    println!("请求成功{}", core::str::from_utf8(& response.data[..response.length]).unwrap());
                }
                Err(e) => {
                    finish_wifi().await;
                    self.loading = false;
                    self.error = Some("请求失败".to_string());
                    println!("请求失败{:?}",e);
                }
            }
            println!("get stack ok" );
        }else{
            self.loading = false;
            self.error = Some("请求失败".to_string());
            println!("get stack err" );
        }
        println!("get stack" );
    }
    async fn sync_time(&mut self) {
        let stack = use_wifi().await;
        if let Ok(v) = stack {
            let sleep_sec = match crate::worldtime::ntp_request(v, get_clock().unwrap()).await {
                Err(_) => {
                    finish_wifi().await;
                    println!("NTP error response");
                }
                Ok(_) => {
                    finish_wifi().await;
                    println!("NTP ok ?");
                },
            };
        }
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

impl Page for ClockPage {
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

        event::on_target(EventType::KeyShort(2),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.request().await;
                println!("count_down_page:{}",mut_ref.choose_index );
            });
        }).await;
        event::on_target(EventType::KeyShort(1),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.sync_time().await;
                println!("count_down_page:{}",mut_ref.choose_index );
            });
        }).await;


        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.back();
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
                        if sync_time_success() {
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
