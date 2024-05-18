use core::cell::RefCell;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use static_cell::make_static;
use crate::pages::count_down_page::CountDownPage;
use crate::pages::main_page::MainPage;

pub mod main_page;
mod count_down_page;


pub trait Page {
    fn new() ->Self;
    fn render(&mut self) -> impl Future<Output=()> +Send +Sync;
   /* fn run(&mut self)-> impl Future<Output=()> +Send +Sync;*/
    async fn  run(&mut self);

}


static MAIN_PAGE:Mutex<CriticalSectionRawMutex,RefCell<Option<MainPage>> > = Mutex::new(RefCell::new(None));
static COUNT_DOWN_PAGE:Mutex<CriticalSectionRawMutex,RefCell<Option<CountDownPage>> > = Mutex::new(RefCell::new(None));

#[embassy_executor::task]
pub async fn main_task(spawner:Spawner){
    MAIN_PAGE.lock().await.get_mut().replace(MainPage::new());
    MainPage::init(spawner).await;
    loop {

       /* let temp = MAIN_PAGE.lock().await;
        let mut mut_ref =  temp.borrow_mut();
        let main_page = mut_ref.as_mut().unwrap();*/
        MainPage::get_mut().await.unwrap().run().await;

        Timer::after(Duration::from_millis(50)).await;
    }
}