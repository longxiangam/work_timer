use alloc::{format, vec};
use alloc::boxed::Box;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::{Dimensions, Point, Size};
use embedded_graphics::prelude::{DrawTarget, Primitive};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use esp_println::{print, println};
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use hal::reset::software_reset;
use heapless::String;
use lcd_drivers::color::TwoBitColor;
use qrcodegen_no_heap::Mask;
use qrcodegen_no_heap::QrCode;
use qrcodegen_no_heap::QrCodeEcc;
use qrcodegen_no_heap::Version;
use crate::display::{display_mut, RENDER_CHANNEL, RenderInfo};
use crate::event;
use crate::event::EventType;
use crate::pages::Page;
use crate::storage::{NvsStorage, WIFI_INFO};
use crate::weather::get_weather;
use crate::widgets::qrcode_widget::QrcodeWidget;
use crate::wifi::{finish_wifi, IP_ADDRESS,  use_wifi,  WIFI_MODEL, WifiNetError};
use crate::web_service::{web_service,STOP_WEB_SERVICE};

pub struct SettingPage {
    need_render:bool,
    running:bool,
    long_start_time:u64,
    ip:String<20>
}

impl SettingPage {


}

impl Page for SettingPage {
    fn new() -> Self {
        Self{
            need_render: false,
            running: false,
            long_start_time: 0,
            ip: Default::default(),
        }
    }

    async fn render(&mut self) {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);
                let ip = unsafe { &IP_ADDRESS };
                let mut url:String<50> = String::new();
                url.push_str("http://");
                url.push_str(ip);
                url.push_str(":8080/config");

                let qrcode_widget = QrcodeWidget::new(&url,Point::new(0,0)
                                                      , Size::new(display.bounding_box().size.height,display.bounding_box().size.height)
                                                      , TwoBitColor::Black, TwoBitColor::White);
                qrcode_widget.draw(display);

                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
            }
        }
    }

    async fn run(&mut self, spawner: Spawner) {
        spawner.spawn(web_service()).ok();
        self.running = true;
        let mut last_time = 0 ;

        loop {
            if !self.running {
                break;
            }
            crate::wifi::refresh_last_time().await;
            self.need_render = true;
            self.render().await;
            Timer::after(Duration::from_millis(50)).await;
        }

        STOP_WEB_SERVICE.signal(());

    }

    async fn bind_event(&mut self) {
        event::clear().await;

        event::on_target(EventType::KeyShort(5),Self::mut_to_ptr(self),  move |ptr|  {
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.running = false;
            });
        }).await;


        event::on_target(EventType::KeyLongStart(5),Self::mut_to_ptr(self),  move |ptr|  {
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                mut_ref.long_start_time = Instant::now().as_secs();
            });
        }).await;

        event::on_target(EventType::KeyLongIng(5),Self::mut_to_ptr(self),  move |ptr|  {
            return Box::pin(async move {
                let mut_ref:&mut Self =  Self::mut_by_ptr(ptr.clone()).unwrap();
                if(mut_ref.long_start_time == 0){
                    mut_ref.long_start_time = Instant::now().as_secs();
                }
                if(Instant::now().as_secs() - mut_ref.long_start_time  > 5){
                    if let Some(wifi_info) = WIFI_INFO.lock().await.as_mut() {
                        println!("wifi_info:{:?}", wifi_info);
                        wifi_info.wifi_finish = false;
                        wifi_info.write();
                        software_reset();
                    }
                }
            });
        }).await;


    }
}


