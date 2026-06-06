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

enum ChargeState {
    One,   //red
    Two,   // green
    Three, // blue
    Four,  // add yellow & flash & wait for rising edge
    Reset, // turn off all
}

static STATE: Signal<CriticalSectionRawMutex, ChargeState> = Signal::new();

#[embassy_executor::task]
async fn some_task(mut s1: ExtiInput<'static>) {
    let mut goto_next = 0;
    loop {
        s1.wait_for_low().await;
        loop {
            let p = select(s1.wait_for_high(), embassy_time::Timer::after_secs(1)).await;
            match p {
                embassy_futures::select::Either::First(_) => {
                    goto_next = 0;
                    STATE.signal(ChargeState::Reset);
                    break;
                }
                embassy_futures::select::Either::Second(_) => {
                    goto_next += 1;
                    match goto_next {
                        1 => STATE.signal(ChargeState::One),
                        2 => STATE.signal(ChargeState::Two),
                        3 => STATE.signal(ChargeState::Three),
                        4 => {
                            STATE.signal(ChargeState::Four);
                            s1.wait_for_high().await;
                            goto_next = 0;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
#[embassy_executor::task]
async fn led_task(
    mut green: Output<'static>,
    mut red: Output<'static>,
    mut blue: Output<'static>,
    mut yellow: Output<'static>,
) {
    loop {
        let s = STATE.wait().await;
        match s {
            ChargeState::One => red.set_high(),
            ChargeState::Two => green.set_high(),
            ChargeState::Three => blue.set_high(),
            ChargeState::Four => {
                yellow.set_high();
                for _ in 0..3 {
                    embassy_time::Timer::after_millis(300).await;
                    green.toggle();
                    yellow.toggle();
                    blue.toggle();
                    red.toggle();
                }
            }
            ChargeState::Reset => {
                yellow.set_low();
                green.set_low();
                blue.set_low();
                red.set_low();
            }
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
    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up); //margine
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up); //dreapta
    spawner.spawn(led_task(green, red, blue, yellow)).unwrap();
    spawner.spawn(some_task(s1)).unwrap();
}
