use core::convert::Infallible;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Duration, TimeoutError, Timer, with_timeout};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::text::{Baseline, Text, TextStyleBuilder};
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};

use hal::dma::Channel0;
use hal::gpio::{Gpio10, Gpio2, Gpio3, Input, Level, Output};
use hal::peripherals::SPI2;
use hal::spi::master::dma::SpiDma;
use lcd_drivers::prelude::Lcd2in7;
use lcd_drivers::prelude::WaveshareDisplay;
use lcd_drivers::color::TwoBitColor;
use lcd_drivers::uc1638::prelude::Display2in7;
use embedded_graphics::{Drawable };
use esp_println::println;
use hal::Async;
use hal::spi::{Error, FullDuplexMode, SpiDataMode, SpiMode};
use lcd_drivers::graphics::TwoBitColorDisplay as _;

pub struct RenderInfo{
    pub time:i32
}

pub static mut DISPLAY:Option<Display2in7>  = None;
pub static RENDER_CHANNEL: Channel<CriticalSectionRawMutex,RenderInfo, 64> = Channel::new();
#[embassy_executor::task]
pub async  fn render(mut spi:  SpiDma<'static, SPI2, Channel0, FullDuplexMode,Async> ,
                           cs:Gpio2 ,// Gpio2<Output<PushPull>>,
                           rst:Gpio10,
                           dc: Gpio3)
{
    let cs = Output::new(cs, Level::High);
    let rst = Output::new( rst, Level::High);
    let dc = Output::new( dc, Level::High);

    let mut spi_device = ExclusiveDevice::new(spi, cs, Delay);

    let mut lcd = Lcd2in7::new(&mut spi_device,dc,rst,&mut Delay).await.unwrap();
    let mut display = Display2in7::default();
    display.clear(TwoBitColor::White);

    let receiver = RENDER_CHANNEL.receiver();
    unsafe {
        DISPLAY.replace(display);
    }

    const PAGE_SIZE: usize = 240;  // 每页的长度
    loop {

        println!("wait render");

        let render_info = receiver.receive().await;

        let buffer = unsafe { DISPLAY.as_mut().unwrap().buffer() };
        let len = buffer.len();
        lcd.goto(&mut spi_device,0,0).await;
        let mut current_page = 0;

        let work = lcd.put_char(&mut spi_device, &buffer);
        println!("begin render");
        match  with_timeout(Duration::from_secs(1),work).await{
            Ok(_) => {
                println!("render success");
            }
            Err(_) => {
                println!("render timeout");
            }
        }

        println!("end render");
        //分页写入
        /*while current_page * PAGE_SIZE < len {
            let start = current_page * PAGE_SIZE;
            let end = usize::min(start + PAGE_SIZE, len);  // 确保不会超出数组长度

            // 传递当前页的数据
            lcd.put_char(&mut spi_device, &buffer[start..end]).await;
            // 如果需要延迟，可以在这里添加
            Timer::after(Duration::from_millis(1)).await;

            current_page += 1;
        }*/

        Timer::after(Duration::from_millis(50)).await;
    }

}



pub fn display_mut()->Option<&'static mut Display2in7>{
    unsafe {
        DISPLAY.as_mut()
    }
}

pub fn draw_text_2(display: &mut Display2in7, text: &str, x: i32, y: i32,color:TwoBitColor) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::iso_8859_16::FONT_9X18)
        .text_color(color)
        .background_color(TwoBitColor::White)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style(text, Point::new(x, y), style, text_style).draw(display);
}