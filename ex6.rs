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
use embassy_sync::signal::Signal;
use panic_probe as _;

static TIME: Signal<CriticalSectionRawMutex, u64> = Signal::new();

#[embassy_executor::task]
async fn time_task(mut s1: ExtiInput<'static>) {
    let mut time_arr = [0_u64; 3];
    let mut t1 = embassy_time::Instant::now();
    let mut count: usize = 1;
    let mut first_time = false;
    loop {
        s1.wait_for_falling_edge().await;
        if first_time {
            t1 = embassy_time::Instant::now();
            continue;
        }
        let t2 = embassy_time::Instant::now();
        let t = t2.duration_since(t1);
        t1 = t2;
        count += 1;
        time_arr[count % 3] = t.as_ticks();
        if count >= 3 {
            let mut avg = 0;
            for x in time_arr {
                avg += x;
            }
            avg /= 3;
            TIME.signal(avg);
        }
    }
}
#[embassy_executor::task]
async fn led_task(mut yellow: Output<'static>) {
    let mut time = TIME.wait().await;
    loop {
        match select(embassy_time::Timer::after_ticks(time), TIME.wait()).await {
            embassy_futures::select::Either::First(_) => {}
            embassy_futures::select::Either::Second(new_time) => time = new_time,
        }
        yellow.toggle();
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

    spawner.spawn(time_task(s1)).unwrap();
    spawner.spawn(led_task(yellow)).unwrap();
}
