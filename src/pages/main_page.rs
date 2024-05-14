use crate::display::display_mut;
//每个page 包含状态与绘制与逻辑处理
struct MainPage{
    count:i32
}

impl Page for  MainPage{
    //通过具体的状态绘制
    fn render() {
        if let Some(display) = display_mut() {

        }
    }
}


pub trait Page{
    fn render();
}



