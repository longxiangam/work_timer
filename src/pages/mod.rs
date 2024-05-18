use core::cell::RefCell;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_println::println;
use static_cell::make_static;
use crate::pages::count_down_page::CountDownPage;
use crate::pages::main_page::MainPage;

pub mod main_page;
mod count_down_page;


enum PageEnum {
    MainPage=0,
    CountDownPage=1,

}



pub trait Page {
    fn new() ->Self;
    fn render(&mut self) -> impl Future<Output=()> +Send +Sync;
   /* fn run(&mut self)-> impl Future<Output=()> +Send +Sync;*/
    async fn  run(&mut self,spawner: Spawner);

}



#[embassy_executor::task]
pub async fn main_task(spawner:Spawner){

    MainPage::init(spawner).await;
    loop {

        MainPage::get_mut().await.unwrap().run(spawner).await;

        Timer::after(Duration::from_millis(50)).await;
    }
}