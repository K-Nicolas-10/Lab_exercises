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

    red.set_low();
    blue.set_low();
    green.set_high();
    yellow.set_high();

    loop {
        if s4.is_high() {
            let level = if s1.is_low() { Level::High } else { Level::Low };
            red.set_level(level);
            green.set_level(s1.get_level());
            blue.set_low();
            yellow.set_high();
        } else {
            let level = if s1.is_low() { Level::High } else { Level::Low };
            blue.set_level(level);
            yellow.set_level(s1.get_level());
        }
        select(
            s1.wait_for_any_edge(),
            s4.wait_for_any_edge(), // Optional: monitor other pins if required
        )
        .await;
        embassy_time::Timer::after_millis(30).await;
    }
}
