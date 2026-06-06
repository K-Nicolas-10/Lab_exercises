#![no_std]
#![no_main]

use defmt::{Format, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use panic_probe as _;
enum State {
    Correct, // sequence good -> Green
    Lock,    //sequence bad -> Red
    Reset,   // releasing all buttons -> Probably turn off all
}

static SEQUENCE: Channel<CriticalSectionRawMutex, State, 8> = Channel::new();

#[embassy_executor::task]
async fn sequence_task(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    loop {
        let mut correct = false;
        loop {
            let p1 = select(s1.wait_for_falling_edge(), s2.wait_for_falling_edge()).await;

            match p1 {
                embassy_futures::select::Either::First(_) => {}
                embassy_futures::select::Either::Second(_) => {
                    SEQUENCE.send(State::Lock).await;
                    break;
                }
            }
            let p2 = select(s1.wait_for_rising_edge(), s2.wait_for_falling_edge()).await;

            match p2 {
                embassy_futures::select::Either::Second(_) => {}
                embassy_futures::select::Either::First(_) => {
                    SEQUENCE.send(State::Lock).await;
                    break;
                }
            }
            let p3 = select(s1.wait_for_rising_edge(), s2.wait_for_rising_edge()).await;

            match p3 {
                embassy_futures::select::Either::First(_) => {}
                embassy_futures::select::Either::Second(_) => {
                    SEQUENCE.send(State::Lock).await;
                    break;
                }
            }

            let p4 = select(s2.wait_for_rising_edge(), s1.wait_for_falling_edge()).await;

            match p4 {
                embassy_futures::select::Either::First(_) => {}
                embassy_futures::select::Either::Second(_) => {
                    SEQUENCE.send(State::Lock).await;
                    break;
                }
            }
            SEQUENCE.send(State::Correct).await;
            break;
        }
        s1.wait_for_high().await;
        s2.wait_for_high().await;

        SEQUENCE.send(State::Reset).await;
    }
}

#[embassy_executor::task]
async fn state_task(mut red: Output<'static>, mut green: Output<'static>) {
    loop {
        let state = SEQUENCE.receive().await;
        match state {
            State::Correct => {
                green.set_high();
                red.set_low();
                embassy_time::Timer::after_millis(500).await;
            }
            State::Lock => {
                red.set_high();
                green.set_low();
            }
            State::Reset => {
                green.set_low();
                red.set_low();
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let green = Output::new(p.PB6, Level::Low, Speed::Medium);
    let red = Output::new(p.PB7, Level::Low, Speed::Medium);
    let mut blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    let yellow = Output::new(p.PC9, Level::Low, Speed::Medium);

    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up);
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up);
    spawner.spawn(state_task(red, green)).unwrap();
    spawner.spawn(sequence_task(s1, s2)).unwrap();
}
