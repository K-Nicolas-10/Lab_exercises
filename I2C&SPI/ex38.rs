#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self, I2c},
    peripherals,
};
use embassy_time::Timer;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

const BMP390_ADDR: u8 = 0x76;
#[embassy_executor::task]
async fn burst_read_task(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
) {
    i2c.write(BMP390_ADDR, &[0x1B, 0b0011_0011]).await.unwrap();
    let mut rx_buf = [0u8; 6];
    i2c.write_read(BMP390_ADDR, &[0x04], &mut rx_buf)
        .await
        .unwrap();
    let raw_pressure =
        ((rx_buf[2] as i32) << 16) | ((rx_buf[1] as i32) << 8) | ((rx_buf[0] as i32) << 0);
    let raw_temp =
        ((rx_buf[5] as i32) << 16) | ((rx_buf[4] as i32) << 8) | ((rx_buf[3] as i32) << 0);
    info!("raw_pressure: {} \n raw_temp: {}", raw_pressure, raw_temp);
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

    spawner.spawn(burst_read_task(i2c)).unwrap();
}
