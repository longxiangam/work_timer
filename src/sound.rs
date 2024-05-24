//处理声音，不使用外部dac ,直接用 pwm 模拟播放提示音够用，

use embassy_time::Delay;
use embedded_hal::delay::DelayNs;
use embassy_time::Timer;
use esp_println::println;
use hal::clock::Clocks;
use hal::dma::DmaDescriptor;
use hal::gpio::{Gpio12, GpioPin, Output, PushPull};
use hal::ledc::channel::Channel;
use hal::ledc::{channel, LEDC, LowSpeed, LSGlobalClkSource, timer};
use hal::peripherals;
use hal::prelude::{_esp_hal_ledc_channel_ChannelHW, _esp_hal_ledc_channel_ChannelIFace, _esp_hal_ledc_timer_TimerIFace, _fugit_RateExtU32};
use static_cell::{make_static, StaticCell};
use wavv::{Samples, Wave};

const bytes: &'static [u8] = include_bytes!("../files/sing8bit.wav");
pub enum  SoundType{
    Warn(u32),
    Music(u32),
    Tips(u32),
}

#[derive(Eq, PartialEq)]
enum PlayerState{
    Playing,
    Stop
}

pub struct PwmPlayer<'a,GPIO: hal::gpio::OutputPin>{
    pwm_channel:Channel<'a,LowSpeed,GPIO>,
    volume:usize,//0-200， 100 是原生
    state:PlayerState
}


impl <'a,GPIO: hal::gpio::OutputPin> PwmPlayer<'a,GPIO> {
    fn init(ledc: peripherals::LEDC, clocks: &'static Clocks<'static>, sound_io:Gpio12<Output<PushPull>>) -> PwmPlayer<'a, GpioPin<Output<PushPull>, 12>> {
        let mut ledc:&mut LEDC<'static>  = make_static!(LEDC::new(ledc, clocks));

        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

        let mut lstimer0 =  make_static!(ledc.get_timer::<LowSpeed>(timer::Number::Timer0));

        lstimer0
            .configure(timer::config::Config {
                duty: timer::config::Duty::Duty8Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: 200u32.kHz(),
            })
            .unwrap();

        let mut channel0 = ledc.get_channel(channel::Number::Channel0, sound_io);
        channel0
            .configure(channel::config::Config {
                timer: lstimer0,
                duty_pct: 10,
                pin_config: channel::config::PinConfig::PushPull,
            })
            .unwrap();


        PwmPlayer{
            pwm_channel: channel0,
            volume: 0,
            state: PlayerState::Stop,
        }
    }

    pub async fn player(&mut self,sound_type: SoundType){
        let wave = Wave::from_bytes(&bytes).unwrap();

        let sample_rate = wave.header.sample_rate;
        let bit_depth = wave.header.bit_depth;
        let num_channels = wave.header.num_channels;

        let pwm_bit = 8;
        let mut i = 1;
        match wave.data {
            Samples::BitDepth8(samples) => {
                self.state = PlayerState::Playing;
                loop {
                    if self.state == PlayerState::Stop {
                        break;
                    }
                    for sample in samples.iter() {

                        self.pwm_channel.set_duty_hw((*sample as f32 *1.3) as u32);
                        //这个延时很重要，通过音频采样率计算
                        Delay.delay_us(125);
                        //Delay.delay_us(62);
                    }
                    Timer::after_secs(1).await;
                }
            },
            Samples::BitDepth16(samples) => println!("{:?}", samples),
            Samples::BitDepth24(samples) => println!("{:?}", samples),
        }

    }

    pub async fn stop(&mut self){
        self.state = PlayerState::Stop;
    }


}


