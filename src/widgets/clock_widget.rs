use alloc::format;
use alloc::string::ToString;
use core::f32::consts::PI;
use micromath::F32Ext;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::prelude::{DrawTargetExt, PixelColor, Primitive, Size};
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_layout::View;
use time::{Date, Month, OffsetDateTime};
use u8g2_fonts::U8g2TextStyle;
use u8g2_fonts::fonts;

#[derive(Eq, PartialEq)]
pub struct ClockWidget<C> {
    pub center: Point,
    pub size: Size,
    datetime: OffsetDateTime,
    front_color: C,
    back_color: C,
}

impl<C> ClockWidget<C> {
    pub fn new(
        center: Point,
        size: Size,
        datetime: OffsetDateTime,
        front_color: C,
        back_color: C,
    ) -> Self {

        Self {
            center,
            size,
            datetime,
            front_color,
            back_color,
        }
    }

}

impl<C> Drawable for ClockWidget<C>
    where
        C: PixelColor,
{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, D::Error>
        where
            D: DrawTarget<Color = Self::Color>,
    {
        // 绘制表盘边界
        let border_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_width(1)
            .build();



        // 计算表盘的半径和中心点
        let radius = self.size.width.min(self.size.height) / 2;
        let center = self.center;


        // 绘制表盘
        Circle::with_center(self.center, radius*2)
            .into_styled(border_style)
            .draw(display);
        // 绘制刻度
        for i in 0..60 {
            let angle = 2.0 * PI * i as f32 / 60.0;
            let inner_radius = if i % 5 == 0 {
                radius as f32 * 0.85
            } else {
                radius as f32 * 0.95
            };
            let outer_radius = radius as f32;
            let inner_point = center + Point::new(
                (inner_radius * angle.cos()) as i32,
                (inner_radius * angle.sin()) as i32,
            );
            let outer_point = center + Point::new(
                (outer_radius * angle.cos()) as i32,
                (outer_radius * angle.sin()) as i32,
            );
            Line::new(inner_point, outer_point)
                .into_styled(border_style)
                .draw(display)?;
        }


        // 获取当前时间的小时、分钟和秒
        let hours = self.datetime.hour() % 12;
        let minutes = self.datetime.minute();
        let seconds = self.datetime.second();

        // 计算每个指针的角度
        // 整圆弧度为 2*pi ，除对应分隔数量得到步进弧度，  -PI/2 是因为时间是从12点位置开始计算，需要减去1/4 个圆弧度
        let hour_angle = 2.0 * PI * (hours as f32 + minutes as f32 / 60.0) / 12.0 - PI / 2.0;
        let minute_angle = 2.0 * PI * (minutes as f32 + seconds as f32 / 60.0) / 60.0 - PI / 2.0;
        let second_angle = 2.0 * PI * seconds as f32 / 60.0 - PI / 2.0;

        // 定义指针的长度
        let hour_hand_length = radius as f32 * 0.5;
        let minute_hand_length = radius as f32 * 0.8;
        let second_hand_length = radius as f32 * 0.9;

        // 定义指针的样式
        let hour_hand_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_width(3)
            .build();

        let minute_hand_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_width(2)
            .build();

        let second_hand_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_width(1)
            .build();

        // 计算并绘制时针
        let hour_hand_end = center + Point::new(
            (hour_hand_length * hour_angle.cos()) as i32,
            (hour_hand_length * hour_angle.sin()) as i32,
        );
        Line::new(center, hour_hand_end)
            .into_styled(hour_hand_style)
            .draw(display)?;

        // 计算并绘制分针
        let minute_hand_end = center + Point::new(
            (minute_hand_length * minute_angle.cos()) as i32,
            (minute_hand_length * minute_angle.sin()) as i32,
        );
        Line::new(center, minute_hand_end)
            .into_styled(minute_hand_style)
            .draw(display)?;

        // 计算并绘制秒针
        let second_hand_end = center + Point::new(
            (second_hand_length * second_angle.cos()) as i32,
            (second_hand_length * second_angle.sin()) as i32,
        );
        Line::new(center, second_hand_end)
            .into_styled(second_hand_style)
            .draw(display)?;


        Ok(())
    }
}
