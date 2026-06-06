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

async fn flash_seq(
    yellow: &mut Output<'static>,
    blue: &mut Output<'static>,
    red: &mut Output<'static>,
    green: &mut Output<'static>,
    seq: &[u64],
    len: usize,
) {
    for led_index in 0..len {
        match seq[led_index] {
            0 => yellow.set_high(),
            1 => blue.set_high(),
            2 => red.set_high(),
            3 => green.set_high(),
            _ => {}
        }
        embassy_time::Timer::after_millis(500).await;
        yellow.set_low();
        blue.set_low();
        red.set_low();
        green.set_low();
        embassy_time::Timer::after_millis(150).await;
    }
}
async fn play_game_over(
    yellow: &mut Output<'static>,
    blue: &mut Output<'static>,
    red: &mut Output<'static>,
    green: &mut Output<'static>,
) {
    for _ in 0..3 {
        yellow.set_high();
        blue.set_high();
        red.set_high();
        green.set_high();
        embassy_time::Timer::after_millis(300).await;
        yellow.set_low();
        blue.set_low();
        red.set_low();
        green.set_low();
        embassy_time::Timer::after_millis(200).await;
    }
}
#[embassy_executor::task]
async fn game_task(
    mut s1: ExtiInput<'static>,
    mut s2: ExtiInput<'static>,
    mut s3: ExtiInput<'static>,
    mut s4: ExtiInput<'static>,
    mut yellow: Output<'static>,
    mut blue: Output<'static>,
    mut red: Output<'static>,
    mut green: Output<'static>,
) {
    let mut random: u64 = 0;
    let t_start = embassy_time::Instant::now();
    loop {
        s1.wait_for_high().await;
        let t_stop = embassy_time::Instant::now();
        random = (t_stop - t_start).as_ticks();
        let mut seq = [0u64; 18];
        let mut steps = 0;
        let mut power = 2;
        let mut game_over = false;
        loop {
            seq[steps] = random / power % 4;
            steps += 1;
            power *= 2;

            flash_seq(
                &mut yellow,
                &mut blue,
                &mut red,
                &mut green,
                &mut seq,
                steps,
            )
            .await;

            for i in 0..steps {
                let press = embassy_futures::select::select4(
                    s1.wait_for_falling_edge(),
                    s2.wait_for_falling_edge(),
                    s3.wait_for_falling_edge(),
                    s4.wait_for_falling_edge(),
                )
                .await;
                embassy_time::Timer::after_millis(30).await;
                let p = match press {
                    embassy_futures::select::Either4::First(_) => 0,
                    embassy_futures::select::Either4::Second(_) => 1,
                    embassy_futures::select::Either4::Third(_) => 2,
                    embassy_futures::select::Either4::Fourth(_) => 3,
                };
                embassy_time::Timer::after_millis(40).await;
                if p != seq[i] {
                    game_over = true;
                    break;
                }
            }
            if game_over {
                game_over = false;
                play_game_over(&mut yellow, &mut blue, &mut red, &mut green).await;
                break;
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

    //margne stange -> dreapta
    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up); // 2
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up); //1
    let mut s3 = ExtiInput::new(p.PA8, p.EXTI8, Pull::Up);
    let mut s4 = ExtiInput::new(p.PB10, p.EXTI10, Pull::Up);

    spawner
        .spawn(game_task(s2, s1, s3, s4, yellow, blue, red, green))
        .unwrap();
    // s1 - yellow
    // s2 -blue
    // s3 - red
    // s4 - green
}
