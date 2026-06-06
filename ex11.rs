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
enum ShiftAction {
    Left,
    Right,
}

static ACTION_CHANNEL: Channel<CriticalSectionRawMutex, ShiftAction, 4> = Channel::new();

#[embassy_executor::task]
async fn direction_task(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    loop {
        match select(s1.wait_for_falling_edge(), s2.wait_for_falling_edge()).await {
            embassy_futures::select::Either::First(_) => {
                ACTION_CHANNEL.send(ShiftAction::Left).await;
            }
            embassy_futures::select::Either::Second(_) => {
                ACTION_CHANNEL.send(ShiftAction::Right).await;
            }
        }
        embassy_time::Timer::after_millis(30).await;
    }
}

#[embassy_executor::task]
async fn led_manager_task(
    mut yellow: Output<'static>,
    mut blue: Output<'static>,
    mut red: Output<'static>,
    mut green: Output<'static>,
) {
    let mut current_position: usize = 0;

    yellow.set_high();
    blue.set_low();
    red.set_low();
    green.set_low();

    loop {
        let action = ACTION_CHANNEL.receive().await;
        info!("Received action: {:?}", action);

        match action {
            ShiftAction::Right => {
                current_position = (current_position + 1) % 4;
            }
            ShiftAction::Left => {
                current_position = if current_position == 0 {
                    3
                } else {
                    current_position - 1
                };
            }
        }

        yellow.set_level(if current_position == 0 {
            Level::High
        } else {
            Level::Low
        });
        blue.set_level(if current_position == 1 {
            Level::High
        } else {
            Level::Low
        });
        red.set_level(if current_position == 2 {
            Level::High
        } else {
            Level::Low
        });
        green.set_level(if current_position == 3 {
            Level::High
        } else {
            Level::Low
        });
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let green = Output::new(p.PB6, Level::Low, Speed::Medium);
    let red = Output::new(p.PB7, Level::Low, Speed::Medium);
    let blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    let yellow = Output::new(p.PC9, Level::Low, Speed::Medium);

    let s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up);
    let s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up);

    spawner.spawn(direction_task(s1, s2)).unwrap();
    spawner
        .spawn(led_manager_task(yellow, blue, red, green))
        .unwrap();
}
