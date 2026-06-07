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

#[embassy_executor::task]
async fn some_task(
    mut adc: Adc<'static, embassy_stm32::peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<embassy_stm32::peripherals::ADC1>,
) {
    const MAX_VALUE: u32 = resolution_to_max_count(Resolution::BITS14);
    const V_REF: f32 = 3.3;
    loop {
        let level: u16 = adc.blocking_read(&mut adc_pin);
        let voltage: f32 = (level as f32 * V_REF) / MAX_VALUE as f32;
        info!("Voltage: {}", voltage);
        Timer::after_millis(500).await;
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
        .spawn(some_task(adc, adc_pin.degrade_adc()))
        .unwrap();
}
