use alloc::{format, vec};
use alloc::boxed::Box;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::{Dimensions, Point, Size};
use embedded_graphics::prelude::{DrawTarget, Primitive};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use esp_println::{print, println};
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

pub struct QrcodePage{
    need_render:bool,
    running:bool,
}

impl QrcodePage{
    fn draw_qrcode<D>(&self,display: &mut D, str: &str) -> Result<(), D::Error>
        where
            D: DrawTarget<Color = TwoBitColor>,
    {
        let qrcode_widget = QrcodeWidget::new(str,Point::new(0,0)
                                              , Size::new(display.bounding_box().size.height,display.bounding_box().size.height)
                                              , TwoBitColor::Black, TwoBitColor::White);
        qrcode_widget.draw(display);
/*
        let mut outbuffer = vec![0u8; Version::MAX.buffer_len()];
        let mut tempbuffer = vec![0u8; Version::MAX.buffer_len()];
        let qr = QrCode::encode_text(str,
                                     &mut tempbuffer, &mut outbuffer, QrCodeEcc::Medium,
                                     Version::MIN, Version::MAX, None, true).unwrap();


        let point_size =  display.bounding_box().size.height as i32  / qr.size() ;


        let mut rectangle = Rectangle::new(Point::new(0,0),Size::new(point_size as u32,point_size as u32));
        let black_style = PrimitiveStyleBuilder::new()
            .fill_color(TwoBitColor::Black)
            .build();
        let white_style = PrimitiveStyleBuilder::new()
            .fill_color(TwoBitColor::White)
            .build();


        for row in 0 .. qr.size() {
            for col in  0 .. qr.size() {
                //绘制2x2矩形
                rectangle.top_left = Point::new(row * point_size,col * point_size);
                if qr.get_module(row,col) {
                    rectangle.into_styled(black_style).draw(display);
                } else {
                    rectangle.into_styled(white_style).draw(display);
                }
            }
        }*/

        Ok(())
    }

}

impl Page for QrcodePage{
    fn new() -> Self {
        Self{
            need_render: false,
            running: false,
        }
    }

    async fn render(&mut self) {
        if self.need_render {
            self.need_render = false;
            if let Some(display) = display_mut() {
                let _ = display.clear(TwoBitColor::White);
                let qrcode_widget = QrcodeWidget::new("http://www.baidu.com/abc/deffffffffffffffff",Point::new(0,0)
                                                      , Size::new(display.bounding_box().size.height,display.bounding_box().size.height)
                                                      , TwoBitColor::Black, TwoBitColor::White);
                qrcode_widget.draw(display);

                RENDER_CHANNEL.send(RenderInfo { time: 0 }).await;
            }
        }
    }

    async fn run(&mut self, spawner: Spawner) {
        self.running = true;
        let mut last_time = 0 ;
        loop {
            if !self.running {
                break;
            }
            self.need_render = true;
            self.render().await;
            Timer::after(Duration::from_millis(50)).await;
        }
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


