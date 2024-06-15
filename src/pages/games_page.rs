use alloc::boxed::Box;
use core::future::Future;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{Drawable, Pixel};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::Point;
use esp_println::println;
use lcd_drivers::color::TwoBitColor;
use crate::chip8::Chip8;
use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::event;
use crate::event::EventType;
use crate::pages::Page;

const ROM: &'static [u8] = include_bytes!("../../roms/INVADERS");
pub struct GamesPage{
    chip8:Chip8,
    running:bool,
}

impl Page for GamesPage{
    fn new() -> Self {
        let mut chip8 = Chip8::new();
        chip8.reset();
        chip8.load_rom(ROM);
        Self{
            chip8,
            running:true,
        }
    }

    async  fn render(&mut self) {
        if let Some(display) = display_mut() {
            println!("chip8 render");
            let _ =  display.clear(TwoBitColor::White);
            for (index_pixel, pixel) in self.chip8.get_display().iter().enumerate() {
                let row: i32 = index_pixel as i32 / 64;
                let col: i32 = index_pixel as i32 % 64;


                if *pixel != 0 {
                    for ii in 0..9 {
                        let _ =  Pixel(Point::new(col * 3 + ii % 3, row * 3 + ii / 3), TwoBitColor::Black).draw(display);
                    }
                } else {
                    for ii in 0..9 {
                        let _ =  Pixel(Point::new(col * 3 + ii % 3, row * 3 + ii / 3), TwoBitColor::White).draw(display);
                    }
                }
            }
            RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
        }
    }

    async fn run(&mut self, spawner: Spawner) {

        self.running = true;
        self.chip8.reset();
        self.chip8.load_rom(ROM);
        loop {

            if !self.running {
                break;
            }
            self.chip8.run();
            self.render().await;
            println!("chip8 running");

            Timer::after(Duration::from_millis(5)).await;
        }

    }

    async fn bind_event(&mut self) {
        event::clear().await;
        event::on_target(EventType::WheelFront,Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin( async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.chip8.key_down(6);
                Timer::after(Duration::from_millis(100)).await;
                mut_ref.chip8.key_up(6);

            });
        }).await;

        event::on_target(EventType::WheelBack,Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin( async  move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.chip8.key_down(4);
                Timer::after(Duration::from_millis(100)).await;
                mut_ref.chip8.key_up(4);

            });
        }).await;
        event::on_target(EventType::KeyShort(1),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin( async  move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr.clone()).unwrap();
                mut_ref.chip8.key_down(5);
                Timer::after(Duration::from_millis(100)).await;
                mut_ref.chip8.key_up(5);
            });
        }).await;
        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |info|  {
            println!("current_page:" );
            return Box::pin( async  move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(info.ptr).unwrap();
                mut_ref.running = false;

            });
        }).await;
    }
}