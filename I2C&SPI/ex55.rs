const BMP390_ADDR: u8 = 0x76;

static TEMP: Signal<CriticalSectionRawMutex, f32> = Signal::new();
#[embassy_executor::task]
async fn led_task(mut red: Output<'static>, mut blue: Output<'static>, mut green: Output<'static>) {
    let mut current_temperature = TEMP.wait().await;
    let mut prev_temp;
    loop {
        prev_temp = current_temperature;
        current_temperature = TEMP.wait().await;
        if current_temperature >= prev_temp + 0.2 {
            red.set_high();
            green.set_low();
            blue.set_low();
        } else if current_temperature <= prev_temp - 0.2 {
            red.set_low();
            green.set_low();
            blue.set_high();
        } else {
            red.set_low();
            green.set_high();
            blue.set_low();
        }
    }
}
#[embassy_executor::task]
async fn temp_task(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
) {
    i2c.write(BMP390_ADDR, &[0x1B, 0x33]).await.unwrap();
    Timer::after_millis(50).await;
    let mut nvm = [0u8; 6];
    i2c.write_read(BMP390_ADDR, &[0x31], &mut nvm)
        .await
        .unwrap();
    let nvm_par_t1 = u16::from_le_bytes([nvm[0], nvm[1]]);
    let nvm_par_t2 = u16::from_le_bytes([nvm[2], nvm[3]]);
    let nvm_par_t3 = nvm[4] as i8;
    let par_t1 = (nvm_par_t1 as f32) / 0.00390625; // 2^-8
    let par_t2 = (nvm_par_t2 as f32) / 1073741824.0; // 2^30
    let par_t3 = (nvm_par_t3 as f32) / 281474976710656.0; // 2^48
    loop {
        let mut rx_buf = [0u8; 3];
        i2c.write_read(BMP390_ADDR, &[0x07], &mut rx_buf)
            .await
            .unwrap();
        let raw_temp = ((rx_buf[2] as u32) << 16) | ((rx_buf[1] as u32) << 8) | (rx_buf[0] as u32);
        let partial_data1 = (raw_temp as f32) - par_t1;
        let partial_data2 = partial_data1 * par_t2;
        let t_lin = partial_data2 + (partial_data1 * partial_data1) * par_t3;
        TEMP.signal(t_lin);
        Timer::after_secs(2).await;
    }
}