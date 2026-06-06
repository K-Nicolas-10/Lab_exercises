#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_stm32::interrupt::EXTI6;
use embassy_stm32::sai::word::U4;
use embassy_stm32::{adc::Adc, exti::ExtiInput, gpio::*, timer::low_level::Timer};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use panic_probe as _;
enum Button {
    Up,
    Down,
}
static BUTTON: Signal<CriticalSectionRawMutex, Button> = Signal::new();
static COUNT: Signal<CriticalSectionRawMutex, u8> = Signal::new();
#[embassy_executor::task]
async fn direction_task(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    s1.wait_for_high().await;
    loop {
        let s = select(s1.wait_for_falling_edge(), s2.wait_for_rising_edge()).await;
        match s {
            embassy_futures::select::Either::First(_) => {
                BUTTON.signal(Button::Up);
            }
            embassy_futures::select::Either::Second(_) => {
                BUTTON.signal(Button::Down);
            }
        }
    }
}

#[embassy_executor::task]
async fn counter_task() {
    loop {
        let (mut count, end, dir): (i8, i8, i8) = match BUTTON.wait().await {
            Button::Up => (0, 15, 1),
            Button::Down => (15, 0, -1),
        };

        loop {
            COUNT.signal(count as u8);

            if count == end {
                break;
            }

            embassy_time::Timer::after_millis(500).await;
            count += dir;
        }
    }
}

#[embassy_executor::task]
async fn led_task(
    mut b3: Output<'static>,
    mut b2: Output<'static>,
    mut b1: Output<'static>,
    mut b0: Output<'static>,
) {
    // MSB  ->  LSB
    // b0 b1 b2 b3
    loop {
        let count = COUNT.wait().await;
        if (count >> 0 & 1) != 0 {
            // b0
            b0.set_high();
        } else {
            b0.set_low();
        }
        if (count >> 1 & 1) != 0 {
            b1.set_high();
        } else {
            b1.set_low();
        }
        if (count >> 2 & 1) != 0 {
            b2.set_high();
        } else {
            b2.set_low();
        }
        if (count >> 3 & 1) != 0 {
            b3.set_high();
        } else {
            b3.set_low();
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
    let mut b_one = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up);
    let mut b_two = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up);

    // Starting at 0
    green.set_low();
    red.set_low();
    blue.set_low();
    yellow.set_low();

    spawner.spawn(led_task(green, red, blue, yellow)).unwrap();
    spawner.spawn(counter_task()).unwrap();
    spawner.spawn(direction_task(b_two, b_one)).unwrap();
}
