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

#[embassy_executor::task]
async fn task_one(mut led_channel: SimplePwmChannel<'static, embassy_stm32::peripherals::TIM3>) {
    let mut led = led_channel;
    led.enable();
    led.set_duty_cycle_percent(25);
    let mut duty = 0;
    loop {
        Timer::after_secs(1).await;
        led.set_duty_cycle_percent(duty);

        duty += 10;
        if duty > 100 {
            duty = 0;
        }
    }
}

const MAX_VALUE: u16 = resolution_to_max_count(Resolution::BITS14) as u16;

#[embassy_executor::task]
async fn task_two(
    mut led_channel: SimplePwmChannel<'static, embassy_stm32::peripherals::TIM3>,
    mut adc: Adc<'static, embassy_stm32::peripherals::ADC1>,
    mut adc_pin: AnyAdcChannel<embassy_stm32::peripherals::ADC1>,
) {
    adc.set_resolution(Resolution::BITS14);
    adc.set_averaging(Averaging::Samples1024);
    adc.set_sample_time(SampleTime::CYCLES160_5);
    let mut led = led_channel;
    led.enable();
    loop {
        let duty = ((adc.blocking_read(&mut adc_pin) as f32 / MAX_VALUE as f32) * 100.0) as u8;
        led.set_duty_cycle_percent(duty);
        info!("duty: {}", duty);
    }
}
#[derive(Clone, Copy)]
enum State {
    Red,
    Yellow,
    Blue,
}

impl State {
    fn next(self) -> Self {
        match self {
            State::Red => State::Yellow,
            State::Yellow => State::Blue,
            State::Blue => State::Red,
        }
    }
}

fn set_rgb(
    r: &mut SimplePwmChannel<'static, embassy_stm32::peripherals::TIM15>,
    g: &mut SimplePwmChannel<'static, embassy_stm32::peripherals::TIM15>,
    b: &mut SimplePwmChannel<'static, embassy_stm32::peripherals::TIM3>,
    state: State,
) {
    let (rv, gv, bv) = match state {
        State::Red => (100, 0, 0),
        State::Yellow => (100, 100, 0),
        State::Blue => (0, 0, 100),
    };

    r.set_duty_cycle_percent(rv);
    g.set_duty_cycle_percent(gv);
    b.set_duty_cycle_percent(bv);
}

#[embassy_executor::task]
async fn task_three(
    mut r: SimplePwmChannel<'static, embassy_stm32::peripherals::TIM15>,
    mut g: SimplePwmChannel<'static, embassy_stm32::peripherals::TIM15>,
    mut b: SimplePwmChannel<'static, embassy_stm32::peripherals::TIM3>,
    mut s4: ExtiInput<'static>,
) {
    let mut current_state = State::Red;
    r.set_polarity(OutputPolarity::ActiveLow);
    g.set_polarity(OutputPolarity::ActiveLow);
    b.set_polarity(OutputPolarity::ActiveLow);

    r.enable();
    g.enable();
    b.enable();

    // Actually show the initial state.
    set_rgb(&mut r, &mut g, &mut b, current_state);

    loop {
        // Active-low button press.
        s4.wait_for_falling_edge().await;

        // Debounce press.
        Timer::after_millis(30).await;

        // Optional but recommended: ignore glitches.
        if s4.is_high() {
            continue;
        }

        current_state = current_state.next();
        set_rgb(&mut r, &mut g, &mut b, current_state);

        // Wait until released so one physical press gives one transition.
        s4.wait_for_rising_edge().await;

        // Debounce release.
        Timer::after_millis(30).await;
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

    spawner
        .spawn(task_three(g_r.ch2, g_r.ch1, b.ch3, s4))
        .unwrap();
}
