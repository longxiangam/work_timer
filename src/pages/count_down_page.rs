use core::future::Future;
use crate::pages::Page;

pub struct  CountDownPage{
    begin_count:u32,
    current_count:u32,
}

impl Page for CountDownPage{
    fn new() -> Self {
        Self{
            begin_count:0,
            current_count:0
        }
    }

    async fn render(&self)  {
        todo!()
    }

    async fn run(&mut self) {
        todo!()
    }
}