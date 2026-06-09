#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::select::Either;
use embassy_stm32::{
    bind_interrupts,
    exti::ExtiInput,
    gpio::Output,
    i2c::{self, I2c},
    peripherals,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

const BMP390_ADDR: u8 = 0x76;

static TEMP: Signal<CriticalSectionRawMutex, f32> = Signal::new();

#[embassy_executor::task]
async fn button_task(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    let mut t = 24.;
    loop {
        let p =
            embassy_futures::select::select(s1.wait_for_falling_edge(), s2.wait_for_falling_edge())
                .await;
        match p {
            Either::First(_) => {
                t += 0.5;
                TEMP.signal(t);
            }
            Either::Second(_) => {
                t -= 0.5;
                TEMP.signal(t);
            }
        }
        Timer::after_millis(30).await;
    }
}

#[embassy_executor::task]
async fn burst_read_task(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
    mut red: Output<'static>,
    mut blue: Output<'static>,
    mut green: Output<'static>,
) {
    i2c.write(BMP390_ADDR, &[0x1B, 0b0011_0011]).await.unwrap();
    Timer::after_millis(30).await;
    let mut nvm = [0u8; 5];
    i2c.write_read(BMP390_ADDR, &[0x31], &mut nvm)
        .await
        .unwrap();

    let nvm_par_t1 = u16::from_le_bytes([nvm[0], nvm[1]]);
    let nvm_par_t2 = u16::from_le_bytes([nvm[2], nvm[3]]);
    let nvm_par_t3 = nvm[4] as i8;

    let par_t1 = (nvm_par_t1 as f32) / 0.00390625; // 2^-8
    let par_t2 = (nvm_par_t2 as f32) / 1073741824.0; // 2^30
    let par_t3 = (nvm_par_t3 as f32) / 281474976710656.0; // 2^48
    let mut t = 24.;
    loop {
        let mut rx_buf = [0u8; 6];
        i2c.write_read(BMP390_ADDR, &[0x04], &mut rx_buf)
            .await
            .unwrap();
        let raw_temp =
            ((rx_buf[5] as i32) << 16) | ((rx_buf[4] as i32) << 8) | ((rx_buf[3] as i32) << 0);
        let partial_data1 = (raw_temp as f32) - par_t1;
        let partial_data2 = partial_data1 * par_t2;
        let t_lin = partial_data2 + (partial_data1 * partial_data1) * par_t3;
        match TEMP.try_take() {
            None => {}
            Some(x) => {
                t = x;
            }
        }
        if t_lin >= t - 0.5 && t_lin <= t + 0.5 {
            blue.set_low();
            red.set_low();
            green.set_high();
        } else if t_lin < t {
            blue.set_low();
            red.set_high();
            green.set_low();
        } else if t_lin > t {
            blue.set_high();
            red.set_low();
            green.set_low();
        }
        info!(
            "target: {} °C, actual: {} °C, raw_temp: {}",
            t, t_lin, raw_temp
        );
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut i2c = I2c::new(
        p.I2C1,
        p.PB6, // SCL
        p.PB7, // SDA
        Irqs,
        p.GPDMA1_CH0,
        p.GPDMA1_CH1,
        Default::default(),
    );
    spawner.spawn(button_task(s1, s2)).unwrap();
    spawner
        .spawn(burst_read_task(i2c, red, blue, green))
        .unwrap();
}
