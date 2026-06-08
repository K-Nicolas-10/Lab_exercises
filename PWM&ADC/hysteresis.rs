#![no_std]
#![no_main]

use cortex_m::interrupt::CriticalSection;
use defmt::{debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::{Adc, AdcChannel, AnyAdcChannel, Resolution, SampleTime},
    gpio::{Level, Output, OutputType, Speed},
    peripherals::{self, TIM2},
    time::hz,
    timer::{
        Ch2,
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_sync::{
    blocking_mutex::{CriticalSectionMutex, raw::CriticalSectionRawMutex},
    signal::Signal,
};
use embassy_time::Timer;
use panic_probe as _;

#[derive(Clone, Copy)]
enum State {
    Dark,
    Bright,
    Freeze,
}
const MAX_VALUE: u32 = embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14);
static STATE: Signal<CriticalSectionRawMutex, State> = Signal::new();
#[embassy_executor::task]
async fn led_task(mut blue: Output<'static>) {
    loop {
        let s = STATE.wait().await;
        match s {
            State::Dark => blue.set_high(),
            State::Bright => blue.set_low(),
            State::Freeze => {}
        }
    }
}
#[embassy_executor::task]
async fn hysteresis_task(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut photo_pin: AnyAdcChannel<peripherals::ADC1>,
) {
    adc.set_averaging(embassy_stm32::adc::Averaging::Disabled);
    adc.set_resolution(Resolution::BITS14);
    adc.set_sample_time(SampleTime::CYCLES160_5);
    loop {
        let adc_percent = (adc.blocking_read(&mut photo_pin) as u32 * 100) / MAX_VALUE;
        match adc_percent {
            0..30 => {
                STATE.signal(State::Dark);
            }
            30..50 => {
                STATE.signal(State::Freeze);
            }
            _ => {
                STATE.signal(State::Bright);
            }
        }
        Timer::after_millis(20).await;
    }
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let adc = Adc::new(p.ADC1);
    let photo_pin = p.PA1;
    let blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    spawner
        .spawn(hysteresis_task(adc, photo_pin.degrade_adc()))
        .unwrap();
    spawner.spawn(led_task(blue)).unwrap();
}
