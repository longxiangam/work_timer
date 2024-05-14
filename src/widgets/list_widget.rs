use alloc::vec::Vec;
use core::marker::PhantomData;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::{PixelColor, Point, Size};

//每个widget 包含状态与绘制，widget 没有业务逻辑只有通过对应事件回调触发
pub struct ListWidget<C>{
    title:&'static str,
    position:Point,
    size:Size,
    items:Vec<ListItemWidget<C>>,
}

impl <C> Drawable for  ListWidget<C> where C:PixelColor{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
        where D: DrawTarget<Color=Self::Color> {
        todo!()
        //显示出所有的item
    }
}


pub struct ListItemWidget<C>{
    label:&'static str,
    position:Point,
    size:Size,
    _marker:PhantomData<C>,
}


impl <C> Drawable for  ListItemWidget<C>  where C:PixelColor{
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
        where D: DrawTarget<Color=Self::Color> {
        todo!()
        //一个中间有方案的矩形
    }
}




pub trait Widget:Drawable{

}