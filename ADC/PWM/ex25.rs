#![no_std]
#![no_main]

use defmt::{Format, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{
    Either3::{self, *},
    select, select3,
};
use embassy_stm32::{
    adc::*,
    lptim::pwm::Pwm,
    pac::exti::Exti,
    peripherals,
    timer::{
        low_level::OutputPolarity,
        simple_pwm::{PwmPin, SimplePwmChannel},
    },
};
use embassy_stm32::{exti::ExtiInput, time::khz};
use embassy_stm32::{gpio::*, timer::simple_pwm::SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use panic_probe as _;

const MAX_VALUE: u16 = embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14) as u16;
//3 sec -> 3000ms
//3000/6 -> 500 duty cycles
// 500 / 2 -> 250
// 250 / 100 -> 2.5

// 1000 steps + 1000
// 2000
//3sec -> 3_000_000 microsex
// 3_000_000 / 2 -> 1_500_000 us / 1000 -> 1500us

#[embassy_executor::task]
async fn breathe_task(mut b: SimplePwmChannel<'static, peripherals::TIM3>) {
    b.enable();
    b.set_polarity(OutputPolarity::ActiveLow);
    b.set_duty_cycle(0);
    let steps = 1000;
    loop {
        for duty in 0..=steps {
            b.set_duty_cycle_fraction(duty, steps);
            Timer::after_micros(1500).await;
        }
        for duty in (0..steps).rev() {
            b.set_duty_cycle_fraction(duty, steps);
            Timer::after_micros(1500).await;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut green = Output::new(p.PB6, Level::Low, Speed::Medium);
    let mut red = Output::new(p.PB7, Level::Low, Speed::Medium);
    let mut blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    //let mut yellow = Output::new(p.PC9, Level::Low, Speed::Medium);

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

    let b: PwmPin<'_, embassy_stm32::peripherals::TIM3, embassy_stm32::timer::Ch3> =
        PwmPin::new(p.PC8, OutputType::PushPull);
    let r: PwmPin<'_, embassy_stm32::peripherals::TIM15, embassy_stm32::timer::Ch2> =
        PwmPin::new(p.PA3, OutputType::PushPull);
    let g: PwmPin<'_, embassy_stm32::peripherals::TIM15, embassy_stm32::timer::Ch1> =
        PwmPin::new(p.PA2, OutputType::PushPull);
    let mut pwm_rgb1 = SimplePwm::new(
        p.TIM3,
        None,
        None,
        Some(b),
        None,
        khz(2),
        Default::default(),
    );
    let mut pwm_rgb = SimplePwm::new(
        p.TIM15,
        Some(g),
        Some(r),
        None,
        None,
        khz(2),
        Default::default(),
    );
    let b = pwm_rgb1.split();
    let g_r = pwm_rgb.split();
    let b = b.ch3;
    let g = g_r.ch1;
    let r = g_r.ch2;

    spawner.spawn(breathe_task(b)).unwrap();
}
