use core::future::Future;

pub mod main_page;
mod count_down_page;


pub trait Page {
    fn new() ->Self;
    fn render(&self) -> impl Future<Output=()> +Send;
    fn run(&mut self)-> impl Future<Output=()> +Send;
}

