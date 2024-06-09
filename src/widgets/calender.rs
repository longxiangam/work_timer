use alloc::format;
use alloc::string::ToString;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::prelude::{PixelColor, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::text::{Baseline, Text, TextStyleBuilder};

use time::{Date, Month};
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

#[derive(Eq, PartialEq)]
pub struct Calender<C>{
    month_first_day: Date,
    month_last_day: Date,
    today:Date,
    front_color:C,
    back_color:C,
}

impl <C> Calender<C>{
   pub fn new(year:i32,month:Month,today:Date, front_color:C, back_color:C)->Self{
       let first_day = Date::from_calendar_date(year, month, 1).unwrap();
       let last_day = (first_day + time::Duration::days(31)).replace_day(1).unwrap().previous_day().unwrap();

       Self{
           month_first_day:first_day,
           month_last_day:last_day,
           today,
           front_color,
           back_color,
       }
   }

    pub fn set_date_of_month(&mut self, year:i32, month:Month){
        let first_day = Date::from_calendar_date(year, month, 1).unwrap();
        let last_day = (first_day + time::Duration::days(31)).replace_day(1).unwrap().previous_day().unwrap();
        self.month_first_day = first_day;
        self.month_last_day = last_day;
    }
}

impl <C> Drawable for Calender<C> where C:PixelColor {
    type Color = C;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, D::Error> where D: DrawTarget<Color=Self::Color> {

        let style =
            U8g2TextStyle::new(fonts::u8g2_font_wqy12_t_gb2312b, self.front_color);
        let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

        let year = self.month_first_day.year();
        let month = self.month_first_day.month();
        // 绘制月份和年份
        let month_year = format!("{}-{}", year, month as u8);
        Text::with_text_style(&month_year, Point::new(0, 0), style.clone(), text_style)
            .draw(display)?;

        // 绘制星期标题
        let days = ["日", "一", "二", "三", "四", "五", "六"];
        for (i, &day) in days.iter().enumerate() {
            Text::with_text_style(day, Point::new(i as i32 * 16, 12), style.clone(), text_style)
                .draw(display)?;
        }

        // 获取当月的第一天和最后一天
        let first_day = Date::from_calendar_date(year, month, 1).unwrap();
        let last_day = (first_day + time::Duration::days(31)).replace_day(1).unwrap().previous_day().unwrap();
        let mut same_month = false;
        let today_day = self.today.day();
        if first_day.year() == self.today.year() && first_day.month() == self.today.month() {
            same_month = true;
        }

        // 绘制日期
        let mut x = first_day.weekday().number_days_from_sunday() as i32 * 16;
        let mut y = 24;
        let line_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_alignment(StrokeAlignment::Inside)
            .stroke_width(1).build();

        for day in 1..=last_day.day() {
            Text::with_text_style(&day.to_string(), Point::new(x, y), style.clone(), text_style)
                .draw(display)?;
            if same_month && day == today_day {
                let rectangle = Rectangle::new(Point::new(x-3,y+3),Size::new(16,12))
                    .into_styled(line_style)
                    .draw(display);
            }

            x += 16;
            if x >= 16 * 7 {
                x = 0;
                y += 12;
            }
        }
        Ok(())
    }
}