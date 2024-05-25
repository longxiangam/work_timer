use embassy_net::{Ipv4Address, Stack};
use embassy_net::tcp::{ConnectError, TcpSocket};
use esp_println::println;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};

const BUFFER_SIZE:usize = 2048;
#[derive(Debug)]
pub enum RequestError{
    TimeOut,
    ConnectError(ConnectError),
    SendError,
    ReadError,
    BufferOver,
}
pub struct RequestClient{
    stack:&'static Stack<WifiDevice<'static,WifiStaDevice>>,
    rx_buffer:[u8;BUFFER_SIZE],
    tx_buffer:[u8;BUFFER_SIZE],
}

pub struct ResponseData {
   pub data:[u8;BUFFER_SIZE],
   pub length:usize,
}


impl RequestClient{
    pub fn new(stack:&'static Stack<WifiDevice<'static,WifiStaDevice>>) -> RequestClient {
        RequestClient{
            stack,
            rx_buffer: [0u8;BUFFER_SIZE],
            tx_buffer: [0u8;BUFFER_SIZE]
        }
    }
    pub async fn send_request(&mut self,url:&str)->Result<ResponseData, RequestError>{
        let mut socket = TcpSocket::new(self.stack, &mut self.rx_buffer, &mut self.tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        //101.35.97.43

        //let remote_endpoint = (Ipv4Address::new(142, 250, 185, 115), 80);
        let remote_endpoint = (Ipv4Address::new(101, 35, 97, 43), 80);
        println!("socket connecting...");
        let r = socket.connect(remote_endpoint).await;
        if let Err(e) = r {
            println!("connect error: {:?}", e);
            return Err(RequestError::ConnectError(e))
        }
        println!("connected!");

        loop {
            use embedded_io_async::Write;
            let r = socket
                .write_all(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
                .await;
            if let Err(e) = r {
                println!("write error: {:?}", e);
                return Err(RequestError::SendError);
            }

            let mut buf = [0_u8; BUFFER_SIZE];
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    return Ok(crate::request::ResponseData{ data: buf, length: 0 });
                }
                Ok(n) => {
                    return Ok(crate::request::ResponseData{ data: buf, length: n });
                    //println!("{}", core::str::from_utf8(& self.rx_buffer[..n]).unwrap());
                },
                Err(e) => {
                    println!("read error: {:?}", e);
                    return Err(RequestError::SendError);
                }
            };

        }
    }
}
