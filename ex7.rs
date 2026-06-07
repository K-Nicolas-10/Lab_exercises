#![no_std]
#![no_main]

use defmt::{Format, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::{
    Either3::{self, *},
    select, select3,
};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use panic_probe as _;

enum State {
    Active,
    Inactive,
}

static STATE: Channel<CriticalSectionRawMutex, State, 8> = Channel::new();

#[embassy_executor::task]
async fn led_task(mut green: Output<'static>, mut red: Output<'static>) {
    loop {
        let s = STATE.receive().await;
        match s {
            State::Active => {
                green.set_high();
                red.set_low();
            }
            State::Inactive => {
                red.set_high();
                green.set_low();
            }
        }
    }
}

#[embassy_executor::task]
async fn active_task(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    loop {
        let p = select3(
            s1.wait_for_low(),
            s2.wait_for_low(),
            embassy_time::Timer::after_secs(5),
        )
        .await;
        match p {
            Either3::Third(_) => STATE.send(State::Inactive).await,
            _ => STATE.send(State::Active).await,
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

    //margne stange -> dreapta
    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up); // 2
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up); //1
    let mut s3 = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up);
    let mut s4 = ExtiInput::new(p.PB10, p.EXTI10, Pull::Up);

    spawner.spawn(led_task(green, red)).unwrap();
    spawner.spawn(active_task(s1, s2)).unwrap();
}
