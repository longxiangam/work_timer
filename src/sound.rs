//处理声音，不使用外部dac ,直接用 pwm 模拟播放提示音够用，

use alloc::vec;
use core::marker::PhantomData;
use core::ops::DerefMut;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Delay;
use embedded_hal::delay::DelayNs;
use embedded_hal::pwm::SetDutyCycle;
use esp_println::println;
use hal::clock::Clocks;
use hal::gpio::{Gpio12, GpioPin, Output, PushPull};
use hal::ledc::channel::Channel;
use hal::ledc::{channel, LEDC, LowSpeed, LSGlobalClkSource, timer};
use hal::ledc::timer::{Timer, TimerIFace};
use hal::peripheral::Peripheral;
use hal::peripherals;
use hal::prelude::{_esp_hal_ledc_channel_ChannelHW, _esp_hal_ledc_channel_ChannelIFace, _esp_hal_ledc_timer_TimerIFace, _fugit_RateExtU32};
use heapless::Vec;
use static_cell::{make_static, };
use wavv::{Samples, Wave};

const BYTES: &'static [u8] = include_bytes!("../files/sing8bit.wav");
#[derive(Clone, Copy)]
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


pub static mut PWM_PLAYER:Option<PwmPlayer<GpioPin<Output<PushPull>,13>>> = None;

pub struct PwmPlayer<GPIO: hal::gpio::OutputPin>{
   /* timer: &'static   dyn TimerIFace<LowSpeed>,*/
    ledc:&'static mut LEDC<'static>,
    //pwm_channel:Channel<'a,LowSpeed,GPIO>,
    sound_id:GPIO,
    volume:usize,//0-200， 100 是原生
    state:Mutex<CriticalSectionRawMutex,PlayerState> ,
}


impl <GPIO> PwmPlayer<GPIO>
    where GPIO:  hal::gpio::OutputPin  + Peripheral<P = GPIO>
{
    pub fn init(ledc: peripherals::LEDC, clocks: &'static Clocks<'static>, sound_io:GPIO) -> PwmPlayer<GPIO>
        where <GPIO as Peripheral>::P: hal::gpio::OutputPin
    {
        let ledc:&mut LEDC<'static>  = make_static!(LEDC::new(ledc, clocks));

        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);


        PwmPlayer{
            ledc,
            sound_id:sound_io,
            volume: 0,
            state: Mutex::new(PlayerState::Stop),
        }
    }

    pub async fn player(&mut self,sound_type: SoundType){


         let mut lstimer0: Timer<LowSpeed> =  self.ledc.get_timer::<LowSpeed>(timer::Number::Timer0);

        lstimer0
            .configure(timer::config::Config {
                duty: timer::config::Duty::Duty8Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: 200u32.kHz(),
            })
            .unwrap();

         let mut channel0 = self.ledc.get_channel(channel::Number::Channel0, unsafe{ self.sound_id.clone_unchecked()});
         channel0
             .configure(channel::config::Config {
                 timer: &lstimer0,
                 duty_pct: 10,
                 pin_config: channel::config::PinConfig::PushPull,
             })
             .unwrap();


        let wave = Wave::from_bytes(&BYTES).unwrap();

        let sample_rate = wave.header.sample_rate;
        let bit_depth = wave.header.bit_depth;
        let num_channels = wave.header.num_channels;

        let pwm_bit = 8;
        let mut i = 1;
        match wave.data {
            Samples::BitDepth8(samples) => {
                *self.state.lock().await = PlayerState::Playing;
                loop {
                    if *self.state.lock().await == PlayerState::Stop {
                        break;
                    }
                    for sample in samples.iter() {

                        channel0.set_duty_hw((*sample as f32 *1.3) as u32);
                        //这个延时很重要，通过音频采样率计算
                        Delay.delay_us(125);
                        //Delay.delay_us(62);
                    }
                    //Timer::after_secs(1).await;
                }
            },
            Samples::BitDepth16(samples) => println!("{:?}", samples),
            Samples::BitDepth24(samples) => println!("{:?}", samples),
        }

    }

    pub async fn stop(&mut self){
        *self.state.lock().await = PlayerState::Stop;
    }

    //由gpt 生成的旋律
    const TWO_TIGER: [(u32, u64); 34] = [
        (262, 500), // C4, 500ms
        (294, 500), // D4, 500ms
        (330, 500), // E4, 500ms
        (262, 1000), // C4, 1000ms
        (0, 500),   // Pause, 500ms
        (262, 500), // C4, 500ms
        (294, 500), // D4, 500ms
        (330, 500), // E4, 500ms
        (262, 1000), // C4, 1000ms
        (0, 500),   // Pause, 500ms
        (330, 500), // E4, 500ms
        (349, 500), // F4, 500ms
        (392, 1000), // G4, 1000ms
        (0, 500),   // Pause, 500ms
        (330, 500), // E4, 500ms
        (349, 500), // F4, 500ms
        (392, 1000), // G4, 1000ms
        (0, 500),   // Pause, 500ms
        (392, 500), // G4, 500ms
        (440, 500), // A4, 500ms
        (392, 500), // G4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (262, 1000), // C4, 1000ms
        (0, 500),   // Pause, 500ms
        (392, 500), // G4, 500ms
        (440, 500), // A4, 500ms
        (392, 500), // G4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (262, 1000), // C4, 1000ms
        (0, 500),   // Pause, 500ms
        (294, 500), // D4, 500ms
        (196, 1000), // G3, 1000ms
    ];

    const TWINKLE_TWINKLE: [(u32, u64); 47] = [
        (261, 500), // C4, 500ms
        (261, 500), // C4, 500ms
        (392, 500), // G4, 500ms
        (392, 500), // G4, 500ms
        (440, 500), // A4, 500ms
        (440, 500), // A4, 500ms
        (392, 1000), // G4, 1000ms
        (0, 100),   // Pause, 500ms
        (349, 500), // F4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (330, 500), // E4, 500ms
        (294, 500), // D4, 500ms
        (294, 500), // D4, 500ms
        (261, 1000), // C4, 1000ms
        (0, 100),   // Pause, 500ms
        (392, 500), // G4, 500ms
        (392, 500), // G4, 500ms
        (349, 500), // F4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (330, 500), // E4, 500ms
        (294, 1000), // D4, 1000ms
        (0, 100),   // Pause, 500ms
        (392, 500), // G4, 500ms
        (392, 500), // G4, 500ms
        (349, 500), // F4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (330, 500), // E4, 500ms
        (294, 1000), // D4, 1000ms
        (0, 100),   // Pause, 500ms
        (261, 500), // C4, 500ms
        (261, 500), // C4, 500ms
        (392, 500), // G4, 500ms
        (392, 500), // G4, 500ms
        (440, 500), // A4, 500ms
        (440, 500), // A4, 500ms
        (392, 1000), // G4, 1000ms
        (0, 100),   // Pause, 500ms
        (349, 500), // F4, 500ms
        (349, 500), // F4, 500ms
        (330, 500), // E4, 500ms
        (330, 500), // E4, 500ms
        (294, 500), // D4, 500ms
        (294, 500), // D4, 500ms
        (261, 1000), // C4, 1000ms
    ];

    //buzzer
    pub async fn player_buzzer(&mut self,sound_type: SoundType){
        let mut melody:Vec<(u32,u64),100> = Vec::new();
            match sound_type {
            SoundType::Warn(_) => {}
            SoundType::Music(n) => {
                if n == 0 {
                    melody = Vec::from_slice(&Self::TWO_TIGER).unwrap() ;
                }
                if n == 1 {
                    melody = Vec::from_slice(&Self::TWINKLE_TWINKLE).unwrap();
                }
            }
            SoundType::Tips(_) => {}
        }

        *self.state.lock().await = PlayerState::Playing;

        let mut times = 5;

         'out:loop {

            for (index,&(freq, duration)) in melody.iter().enumerate() {
                let mut lstimer0: Timer<LowSpeed> =  self.ledc.get_timer::<LowSpeed>(timer::Number::Timer0);
                let mut channel0 = self.ledc.get_channel(channel::Number::Channel0, unsafe{ self.sound_id.clone_unchecked()});
                if freq == 0 {
                    // 如果是停顿符，直接延时
                    embassy_time::Timer::after_millis(duration).await;
                } else {
                    lstimer0.configure(timer::config::Config {
                        duty: timer::config::Duty::Duty8Bit,
                        clock_source: timer::LSClockSource::APBClk,
                        frequency: freq * 10.Hz(),
                    });
                    channel0.configure(channel::config::Config {
                        timer: &lstimer0,
                        duty_pct: 10,
                        pin_config: channel::config::PinConfig::PushPull,
                    });
                    channel0.set_duty(50);
                    embassy_time::Timer::after_millis(duration).await;
                    channel0.set_duty(0);
                }
                if *self.state.lock().await == PlayerState::Stop {
                    channel0.set_duty(0);
                    break 'out;
                }

                if index == melody.len()-1 {
                    channel0.set_duty(0);
                }
            }
             times-=1;

             if times == 0 {
                 break;
             }

            embassy_time::Timer::after_secs(3).await;

        }
    }

}



pub async fn player_buzzer(sound_type: SoundType){
    PLAYER_SIGN.signal(sound_type);
}

pub async fn stop_buzzer(){
    unsafe {
        if let Some(ref mut player) = PWM_PLAYER{
            player.stop().await;
        }
    }
}

static PLAYER_SIGN:Signal<CriticalSectionRawMutex,SoundType> = Signal::new();
#[embassy_executor::task]
pub async fn  buzzer_task(){
    loop {
        let sound_type = PLAYER_SIGN.wait().await;
        PLAYER_SIGN.reset();
        unsafe {
            if let Some(ref mut player) = PWM_PLAYER{
                player.player_buzzer(sound_type).await;
            }
        }
    }
}