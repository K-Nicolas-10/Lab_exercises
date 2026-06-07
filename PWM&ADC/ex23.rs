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

#[embassy_executor::task]
async fn filter_task(
    mut adc: Adc<'static, embassy_stm32::peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<embassy_stm32::peripherals::ADC1>,
) {
    let mut first_read = true;
    let mut prev_val: f32 = 0.;
    loop {
        if first_read {
            first_read = false;
            prev_val = adc.blocking_read(&mut adc_pin) as f32;
            info!("Current Reading:{} ", prev_val);
            continue;
        }
        let current_reading = adc.blocking_read(&mut adc_pin);
        let filtered_value = ((ALPHA * current_reading as f32) + ((1.0 - ALPHA) * prev_val as f32));
        prev_val = filtered_value;
        info!(
            "Current Reading:   {} \n Filtered Reading:  {} ",
            current_reading, filtered_value
        );
        Timer::after_millis(10).await;
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

    spawner
        .spawn(filter_task(adc, adc_pin.degrade_adc()))
        .unwrap();
}
