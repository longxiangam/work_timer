use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::{PixelColor, Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};

#[derive(Eq, PartialEq,Debug)]
pub struct BatteryWidget<C>{
    percent:u32,
    front_color:C,
    back_color:C,
    position: Point,
    size: Size,
}

impl <C> BatteryWidget<C>{
    pub fn new(percent:u32,position:Point,size: Size,front_color:C,back_color:C)->Self{
        Self{
            percent,
            front_color,
            back_color,
            position,
            size,
        }
    }
    pub fn set_current_value(&mut self, percent: u32) {
        self.percent = percent;
    }
}

impl <C> Drawable for BatteryWidget<C>   where
    C: PixelColor,{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
        where
            D: DrawTarget<Color = Self::Color>,
    {

        let gap = 2;

        Rectangle::new(self.position, self.size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(self.back_color)
                    .stroke_color(self.front_color)
                    .stroke_width(1)
                    .build(),
            )
            .draw(target)?;

        let terminal_width = 2;
        let terminal_height = self.size.height - 4;
        let terminal_x = self.position.x - terminal_width as i32;
        let terminal_y = self.position.y + 2;

        Rectangle::new(Point::new(terminal_x, terminal_y), Size::new(terminal_width, terminal_height))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(self.front_color)
                    .stroke_color(self.front_color)
                    .stroke_width(1)
                    .build(),
            )
            .draw(target)?;

        let filled_width = ((self.size.width - 4 * gap) * self.percent * 100) as i32;

        if filled_width > 0 {
            Rectangle::new(
                Point::new(self.position.x + gap as i32, self.position.y + gap as i32),
                Size::new(filled_width as u32, self.size.height - 2 * gap),
            )
                .into_styled(PrimitiveStyleBuilder::new().fill_color(self.front_color).build())
                .draw(target)?;
        }

        Ok(())
    }
}
