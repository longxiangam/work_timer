use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::future::Future;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::prelude::{Dimensions, DrawTarget, OriginDimensions};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use embedded_layout::align::Align;
use embedded_layout::layout::linear::Horizontal;
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use u8g2_fonts::{FontRenderer, U8g2TextStyle};
use u8g2_fonts::fonts;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};
use crate::battery::{BATTERY, Battery};
use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::{battery, event};
use crate::event::EventType;
use crate::model::seniverse::{DailyResult, form_json};
use crate::pages::Page;
use crate::request::RequestClient;
use crate::weather::{get_weather, WEATHER_SYNC_SUCCESS};
use crate::widgets::battery_widget::BatteryWidget;
use crate::wifi::{finish_wifi, use_wifi};
use crate::worldtime::{get_clock, sync_time_success};


pub struct WeatherPage{
    weather_data: Option<DailyResult>,
    running:bool,
    need_render:bool,
    loading:bool,
    error:Option<String>,
}


impl WeatherPage{
    fn draw_clock<D>(display: &mut D, time: &str) -> Result<(), D::Error>
        where
            D: DrawTarget<Color = TwoBitColor>,
    {
        let character_style = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(18, 43))
            .segment_width(4)
            .segment_color(TwoBitColor::Black)
            .build();

        let text_style = TextStyleBuilder::new()
            .alignment(Alignment::Left)
            .baseline(Baseline::Top)
            .build();

        Text::with_text_style(
            &time,
            Point::new(0, (display.bounding_box().size.height - 45) as i32),
            character_style,
            text_style,
        )
            .draw(display)?;


        Ok(())
    }

}

impl Page for  WeatherPage{
    fn new() -> Self {
        Self{
            weather_data: None,
            running: false,
            need_render: false,
            loading: false,
            error: None,
        }
    }

    async fn render(&mut self)  {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);

                let style =
                    U8g2TextStyle::new(fonts::u8g2_font_wqy12_t_gb2312b, TwoBitColor::Black);

                if let Some(battery) = BATTERY.lock().await.as_ref() {
                    let mut battery_widget = BatteryWidget::new(0, Point::new((display.size().width - 20) as i32, 2), Size::new(20, 10)
                                                            , TwoBitColor::Black, TwoBitColor::White);

                    battery_widget.set_current_value(battery.percent);
                    battery_widget.draw(display);
                }


                if *WEATHER_SYNC_SUCCESS.lock().await {
                    if let Some(weather) = get_weather() {

                        if let Some(weather) = weather.daily_result.lock().await.as_mut() {

                            let mut y = 10;
                            for one in weather.daily.iter() {
                                let (year, date) = one.date.split_once('-').unwrap();
                                let date = date.replace("-",".");
                                let str = format_args!("{} {}/{},{}/{}℃,湿:{}%,风:{}"
                                                       ,date,one.text_day,one.text_night,one.low,one.high,one.humidity,one.wind_scale).to_string();
                                let _ = Text::new(str.as_str(), Point::new(0, y), style.clone()).draw(display);
                                y+=15;
                            }

                        }
                        else{
                            let _ = Text::new("同步天气...", Point::new(0,50), style.clone())
                                .draw(display);
                        }
                    }

                }
                if sync_time_success() {
                    if let Some(clock) = get_clock() {
                        let local = clock.local().await;
                        let hour = local.hour();
                        let minute = local.minute();
                        let second = local.second();


                        let str = format_args!("{:02}:{:02}:{:02}",hour,minute,second).to_string();

                        let date = clock.get_date_str().await;
                        let week = clock.get_week_day().await;

                        Self::draw_clock(display,str.as_str());

                        let font: FontRenderer = FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>();
                        let date_rect = Rectangle::new(Point::new((display.size().width - 80) as i32, (display.size().height - 35) as i32)
                                                       , Size::new(80,20));
                        let week_rect = Rectangle::new(Point::new((display.size().width - 80) as i32, (display.size().height - 15) as i32)
                                                       , Size::new(80,20));

                        let _ = font.render_aligned(
                            format_args!("{} ",date),
                            date_rect.center(),
                            VerticalPosition::Center,
                            HorizontalAlignment::Center,
                            FontColor::Transparent(TwoBitColor::Black),
                            display,
                        );

                        let _ = font.render_aligned(
                            format_args!("{} ",week),
                            week_rect.center(),
                            VerticalPosition::Center,
                            HorizontalAlignment::Center,
                            FontColor::Transparent(TwoBitColor::Black),
                            display,
                        );

                    }
                }else{
                    Self::draw_clock(display,"同步时间...");
                }
                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;

            }
        }
    }

    async fn run(&mut self, spawner: Spawner) {
        self.running = true;
        if let None = self.weather_data{
            //self.request().await;
        }
        loop {

            if !self.running {
                break;
            }
            self.need_render = true;
            self.render().await;

            Timer::after(Duration::from_millis(50)).await;
        }
    }

    async fn bind_event(&mut self) {
        event::clear().await;
        event::on_target(EventType::KeyShort(1), Self::mut_to_ptr(self), move |ptr| {
            return Box::pin(async move {
                if let Some(weather) = get_weather() {
                    weather.request().await;
                }
            });
        }).await;
        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.running = false;
            });
        }).await;
    }
}

