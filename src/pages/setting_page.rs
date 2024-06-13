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
use crate::weather::get_weather;
use crate::widgets::qrcode_widget::QrcodeWidget;
use crate::wifi::{finish_wifi, IP_ADDRESS, STOP_WEB_SERVICE, use_wifi, web_service, WIFI_MODEL, WifiNetError};

pub struct SettingPage {
    need_render:bool,
    running:bool,
    ip:String<20>
}

impl SettingPage {


}

impl Page for SettingPage {
    fn new() -> Self {
        Self{
            need_render: false,
            running: false,
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
    }
}


