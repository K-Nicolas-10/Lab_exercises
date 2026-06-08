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

#[embassy_executor::task]
async fn but_event(mut s1: ExtiInput<'static>) {
    loop {
        s1.wait_for_falling_edge().await;
        BUTTON_EVENT.send(()).await;
    }
}

#[derive(Copy, Clone)]
enum State {
    EditRed,
    EditGreen,
    EditBlue,
    Final,
}
const MAX_VALUE: u16 = embassy_stm32::adc::resolution_to_max_count(Resolution::BITS14) as u16;
#[embassy_executor::task]
async fn adc_task(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<peripherals::ADC1>,
    mut r: SimplePwmChannel<'static, peripherals::TIM15>,
    mut g: SimplePwmChannel<'static, peripherals::TIM15>,
    mut b: SimplePwmChannel<'static, peripherals::TIM3>,
) {
    adc.set_resolution(Resolution::BITS14);
    adc.set_averaging(Averaging::Samples16);
    adc.set_sample_time(SampleTime::CYCLES160_5);
    r.set_polarity(OutputPolarity::ActiveLow);
    g.set_polarity(OutputPolarity::ActiveLow);
    b.set_polarity(OutputPolarity::ActiveLow);
    r.enable();
    g.enable();
    b.enable();
    let mut state = State::EditRed;
    let mut locked_green: u16 = 0;
    let mut locked_red: u16 = 0;
    let mut locked_blue: u16 = 0;
    loop {
        if BUTTON_EVENT.try_receive().is_ok() {
            state = match state {
                State::EditRed => State::EditGreen,
                State::EditGreen => State::EditBlue,
                State::EditBlue => State::Final,
                State::Final => {
                    locked_green = 0;
                    locked_blue = 0;
                    State::EditRed
                }
            }
        }
        match state {
            State::EditRed => locked_red = adc.blocking_read(&mut adc_pin),
            State::EditGreen => locked_green = adc.blocking_read(&mut adc_pin),
            State::EditBlue => locked_blue = adc.blocking_read(&mut adc_pin),
            State::Final => {}
        }
        r.set_duty_cycle_fraction(locked_red, MAX_VALUE);
        g.set_duty_cycle_fraction(locked_green, MAX_VALUE);
        b.set_duty_cycle_fraction(locked_blue, MAX_VALUE);
        Timer::after_millis(10).await;
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
        .spawn(adc_task(adc, adc_pin.degrade_adc(), r, g, b))
        .unwrap();
    spawner.spawn(but_event(s4)).unwrap();
}
