use alloc::{format, vec};
use alloc::vec::Vec;
use core::marker::PhantomData;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::{PixelColor, Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::{Baseline, Text, TextStyleBuilder};
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use u8g2_fonts::{FontRenderer, U8g2TextStyle};
use u8g2_fonts::fonts;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};

//每个widget 包含状态与绘制，widget 没有业务逻辑只有通过对应事件回调触发
pub struct ListWidget<C>{
    title:&'static str,
    position:Point,
    size:Size,
    front_color:C,
    back_color:C,
    items:Vec<ListItemWidget<C>>,
}


impl <C: Clone> ListWidget<C>{
    pub fn new(position: Point,front_color:C,back_color:C,size: Size,title:&'static str,items:Vec<&'static str>) ->Self{
        let mut list_items = vec![];
        let item_size = Size::new(size.width,20);
        for (index,item) in items.iter().enumerate() {
            let item_position = Point::new(position.x,position.y + (index) as i32 *20 );
            let list_item = ListItemWidget::new(item_position,front_color.clone(),back_color.clone(),item_size,item);
            list_items.push(list_item);
        }
        Self{
            title,
            position ,
            size,
            front_color,
            back_color,
            items: list_items,
        }
    }

    pub fn choose(&mut self,index:usize){
        for (i,item) in self.items.iter_mut().enumerate() {
            if i == index {
                item.is_choose = true;
            } else {
                item.is_choose = false;
            }
        }
    }
}

impl <C> Drawable for  ListWidget<C> where C:PixelColor{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
        where D: DrawTarget<Color=Self::Color> {
        let line_style = PrimitiveStyleBuilder::new()
            .stroke_color(self.front_color)
            .stroke_alignment(StrokeAlignment::Inside)
            .stroke_width(1).build();
        let rectangle = Rectangle::new(self.position,self.size)
            .into_styled(line_style)
            .draw(target);

        for item in self.items.iter() {
            item.draw(target);
        }

        Ok(())
    }
}


pub struct ListItemWidget<C>{
    label:&'static str,
    position:Point,
    size:Size,
    front_color:C,
    back_color:C,
    is_choose:bool,
    _marker:PhantomData<C>,
}

impl <C: Clone>ListItemWidget<C>{
    fn new(position: Point,front_color:C,back_color:C,size: Size,label:&'static str) ->Self{

        Self{
            label,
            position ,
            size,
            front_color,
            back_color,
            is_choose: false,
            _marker: Default::default(),
        }

    }
}


impl <C> Drawable for  ListItemWidget<C>  where C:PixelColor{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
        where D: DrawTarget<Color=Self::Color> {

        if self.is_choose {
            let line_style = PrimitiveStyleBuilder::new()
                .stroke_color(self.front_color)
                .stroke_alignment(StrokeAlignment::Inside)
                .stroke_width(1).build();
            let rectangle = Rectangle::new(self.position,self.size)
                .into_styled(line_style)
                .draw(target);

        }

        println!("绘制项");
       /* let mut style =
            U8g2TextStyle::new(fonts::u8g2_font_wqy12_t_gb2312b, self.front_color);
        Text::new(self.label, self.position, style )
            .draw(target);*/

        let mut tag = "-";
        if self.is_choose {
            tag = "+";
        }


        let font: FontRenderer = FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>();
        font.render_aligned(
            format_args!("{} {}",tag, self.label),
            self.position+Point::new(20,5),
            VerticalPosition::Top,
            HorizontalAlignment::Left,
            FontColor::Transparent(self.front_color),
            target,
        );
        //style.set_background_color(Some(self.back_color));


        //一个中间有方案的矩形
        Ok(())
    }
}




pub trait Widget:Drawable{

}