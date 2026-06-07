#![no_std]
#![no_main]

use defmt::{Format, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{
    Either3::{self, *},
    select, select3,
};
use embassy_stm32::adc::*;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use panic_probe as _;

const ALPHA: f32 = 0.1;
const MAX_VALUE: u32 = resolution_to_max_count(Resolution::BITS14);
const ONE: u16 = (MAX_VALUE as f32 * 0.2) as u16;
const TWO: u16 = (MAX_VALUE as f32 * 0.4) as u16;
const THREE: u16 = (MAX_VALUE as f32 * 0.6) as u16;
const FOUR: u16 = (MAX_VALUE as f32 * 0.8) as u16;
const FIVE: u16 = MAX_VALUE as u16;

enum Zone {
    One,
    Two,
    Three,
    Four,
    Five,
    Unclear,
}

static ZONE: Signal<CriticalSectionRawMutex, Zone> = Signal::new();

#[embassy_executor::task]
async fn adc_task(
    mut adc: Adc<'static, embassy_stm32::peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<embassy_stm32::peripherals::ADC1>,
) {
    loop {
        let val = adc.blocking_read(&mut adc_pin);
        let zone = match val {
            0..ONE => Zone::One,
            ONE..TWO => Zone::Two,
            TWO..THREE => Zone::Three,
            THREE..FOUR => Zone::Four,
            FOUR..=FIVE => Zone::Five,
            _ => Zone::Unclear,
        };
        ZONE.signal(zone);
        Timer::after_millis(5).await;
    }
}

#[embassy_executor::task]
async fn led_task(
    mut yellow: Output<'static>,
    mut blue: Output<'static>,
    mut red: Output<'static>,
    mut green: Output<'static>,
) {
    loop {
        let p = ZONE.wait().await;
        match p {
            Zone::One => {
                yellow.set_low();
                blue.set_low();
                red.set_low();
                green.set_low();
            }
            Zone::Two => {
                yellow.set_high();
                blue.set_low();
                red.set_low();
                green.set_low();
            }
            Zone::Three => {
                yellow.set_high();
                blue.set_high();
                red.set_low();
                green.set_low();
            }
            Zone::Four => {
                yellow.set_high();
                blue.set_high();
                red.set_high();
                green.set_low();
            }
            Zone::Five => {
                yellow.set_high();
                blue.set_high();
                red.set_high();
                green.set_high();
            }
            Zone::Unclear => {}
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut green = Output::new(p.PB6, Level::Low, Speed::Medium);
    let mut red = Output::new(p.PB7, Level::Low, Speed::Medium);
    let mut blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    let mut yellow = Output::new(p.PC9, Level::Low, Speed::Medium);

    //margne stange -> dreapta
    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up); // 2
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up); //1
    let mut s3 = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up);
    let mut s4 = ExtiInput::new(p.PB10, p.EXTI10, Pull::Up);

    let mut adc = Adc::new(p.ADC1);
    adc.set_resolution(Resolution::BITS14);
    adc.set_averaging(Averaging::Samples1024);
    adc.set_sample_time(SampleTime::CYCLES160_5);
    let mut adc_pin = p.PA0;
    spawner.spawn(led_task(yellow, blue, red, green)).unwrap();
    spawner.spawn(adc_task(adc, adc_pin.degrade_adc())).unwrap();
}
