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
const MID_VALUE: u16 = (embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14) / 2) as u16;
#[embassy_executor::task]
async fn adc_task(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<peripherals::ADC1>,
    mut r: SimplePwmChannel<'static, peripherals::TIM15>,
    mut g: SimplePwmChannel<'static, peripherals::TIM15>,
    mut b: SimplePwmChannel<'static, peripherals::TIM3>,
) {
    adc.set_resolution(Resolution::BITS14);
    adc.set_averaging(Averaging::Samples1024);
    adc.set_sample_time(SampleTime::CYCLES160_5);
    r.set_polarity(OutputPolarity::ActiveLow);
    g.set_polarity(OutputPolarity::ActiveLow);
    b.set_polarity(OutputPolarity::ActiveLow);
    r.enable();
    g.enable();
    b.enable();
    loop {
        let val = adc.blocking_read(&mut adc_pin) as u32;
        let max = MAX_VALUE as u32;
        let mid = MID_VALUE as u32;

        let (rv, gv, bv): (u8, u8, u8);

        if val <= mid {
            let t = val * 100 / mid;

            rv = t as u8;
            gv = t as u8;
            bv = 100;
        } else {
            let t = (val - mid) * 100 / (max - mid);

            rv = 100;
            gv = (100 - t / 2) as u8; 
            bv = (100 - t) as u8; 
        }

        r.set_duty_cycle_percent(rv);
        g.set_duty_cycle_percent(gv);
        b.set_duty_cycle_percent(bv);
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
    spawner
        .spawn(adc_task(adc, adc_pin.degrade_adc(), r, g, b))
        .unwrap();
}
