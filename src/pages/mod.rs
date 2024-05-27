use alloc::string::String;
use core::cell::RefCell;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_println::println;
use static_cell::make_static;
use crate::pages::clock_page::ClockPage;
use crate::pages::main_page::MainPage;

pub mod main_page;
mod clock_page;
mod games_page;
mod timer_page;


enum PageEnum {
    EMainPage,
    EClockPage,
    ETimerPage,
    EWeatherPage,
    ECalenderPage,
    EChip8Page,

}
struct  MenuItem{
    page_enum:PageEnum,
    title:String,
}
impl MenuItem{
    fn new(title:String, page_enum: PageEnum) -> MenuItem {
        Self{
            page_enum,
            title,
        }
    }
}


pub trait Page {
    fn new() ->Self;
    fn render(&mut self) -> impl Future<Output=()> +Send +Sync;
   /* fn run(&mut self)-> impl Future<Output=()> +Send +Sync;*/
    async fn  run(&mut self,spawner: Spawner);
    async fn bind_event(&mut self);

    fn mut_by_ptr<'a,T>(ptr:Option<usize>)->Option<&'a mut T>{
        unsafe {
            if let Some(v) =  ptr {
                return Some(&mut *(v as *mut T));
            }else{
                return None;
            }
        }
    }

    fn mut_to_ptr<T>(ref_mut:&mut T)->usize{
        unsafe {
            ref_mut as *mut T as usize
        }
    }
}



#[embassy_executor::task]
pub async fn main_task(spawner:Spawner){

    MainPage::init(spawner).await;
    loop {

        MainPage::get_mut().await.unwrap().run(spawner).await;

        Timer::after(Duration::from_millis(50)).await;
    }
}