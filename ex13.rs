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

#[derive(Clone, Copy, Format)]
struct EdgeEvent {
    is_pressed: bool,
    total_edges: u64,
}
static COUNT: Channel<CriticalSectionRawMutex, EdgeEvent, 8> = Channel::new();

#[embassy_executor::task]
async fn count_task(mut s1: ExtiInput<'static>) {
    let mut count: u64 = 0;
    let mut pressed = false;

    loop {
        s1.wait_for_any_edge().await;
        if s1.is_low() {
            pressed = true;
            count += 1;
        } else {
            pressed = false;
            count += 1;
        }
        COUNT
            .send(EdgeEvent {
                is_pressed: pressed,
                total_edges: count,
            })
            .await;
    }
}

#[embassy_executor::task]
async fn led_task(mut blue: Output<'static>) {
    loop {
        let event = COUNT.receive().await;
        let level: Level = if event.is_pressed {
            Level::High
        } else {
            Level::Low
        };
        blue.set_level(level);
        info!("Pressed: {}", event.total_edges);
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
    let s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up);

    spawner.spawn(led_task(blue)).unwrap();
    spawner.spawn(count_task(s1)).unwrap();
}
