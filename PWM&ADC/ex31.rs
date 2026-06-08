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
    pac::exti::{Exti, regs::Exticr},
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

static BUTTON_EVENT: Channel<CriticalSectionRawMutex, (), 4> = Channel::new();

const MAX_VALUE: u16 = embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14) as u16;

//3 states

const ONE_THIRD: u16 = embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14) as u16 / 3;
const TWO_THIRD: u16 = ONE_THIRD * 2;
const THREE_THIRD: u16 = ONE_THIRD * 3;

static ADC_VALUE: Signal<CriticalSectionRawMutex, u16> = Signal::new();
#[embassy_executor::task]
async fn adc_task(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<peripherals::ADC1>,
    mut y: SimplePwmChannel<'static, peripherals::TIM15>,
) {
    adc.set_resolution(Resolution::BITS14);

    let mut t: u8 = 0;
    let mut up = true;

    y.set_polarity(OutputPolarity::ActiveLow);
    y.enable();

    loop {
        let val = adc.blocking_read(&mut adc_pin) as u32;

        y.set_duty_cycle_percent(t);

        if t >= 100 {
            up = false;
        } else if t == 0 {
            up = true;
        }

        if up {
            t += 1;
        } else {
            t -= 1;
        }

        let delay_ms = 5 + (20 * val) / (MAX_VALUE as u32);
        Timer::after_millis(delay_ms as u64).await;
    }
}

#[embassy_executor::main]

async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    //let mut yellow = Output::new(p.PC9, Level::Low, Speed::Medium);
    //let mut blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    //let mut red = Output::new(p.PB7, Level::Low, Speed::Medium);
    //let mut green = Output::new(p.PB6, Level::Low, Speed::Medium);

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

    let mut blue_pin: PwmPin<'_, embassy_stm32::peripherals::TIM2, embassy_stm32::timer::Ch1> =
        PwmPin::new(p.PA5, OutputType::PushPull);
    let mut yellow_pin: PwmPin<'_, embassy_stm32::peripherals::TIM3, embassy_stm32::timer::Ch4> =
        PwmPin::new(p.PC9, OutputType::PushPull);
    let mut pwm_rgb1 = SimplePwm::new(
        p.TIM3,
        None,
        None,
        Some(b),
        Some(yellow_pin),
        khz(1),
        Default::default(),
    );
    let mut pwm_rgb = SimplePwm::new(
        p.TIM15,
        Some(g),
        Some(r),
        None,
        None,
        khz(1),
        Default::default(),
    );
    let b = pwm_rgb1.split();
    let g_r = pwm_rgb.split();
    let b = b.ch3;
    let g = g_r.ch1;
    let r = g_r.ch2;

    spawner
        .spawn(adc_task(adc, adc_pin.degrade_adc(), r))
        .unwrap();
}
