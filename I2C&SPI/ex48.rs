const BMP390_ADDR: u8 = 0x76;
const EEPROM_ADDR: u8 = 0x50;

async fn read_temp(
    i2c: &mut I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
) -> u32 {
    let mut data = [0u8; 3];
    i2c.write_read(BMP390_ADDR, &[0x07], &mut data)
        .await
        .unwrap();
    let raw_temp = ((data[2] as u32) << 16) | ((data[1] as u32) << 8) | (data[0] as u32);
    raw_temp
}

#[embassy_executor::task]
async fn main_task(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
) {
    let mem_addr: u16 = 0x0000;
    let mem_addr = mem_addr.to_be_bytes();
    let mut data = [0u8; 2];
    i2c.write_read(EEPROM_ADDR, &mem_addr, &mut data)
        .await
        .unwrap();
    Timer::after_millis(5).await;
    let mut max = u8::from_be_bytes([data[0]]);
    let mut min = u8::from_be_bytes([data[1]]);
    if max == 0xFF {
        max = 0;
    }
    if min == 0xFF {
        min = 255;
    }
    i2c.write(BMP390_ADDR, &[0x1B, 0b0011_0011]).await.unwrap();
    Timer::after_millis(5).await;
    let mut nvm_data = [0u8; 5];
    i2c.write_read(BMP390_ADDR, &[0x31], &mut nvm_data)
        .await
        .unwrap();
    let nvm_par_t1: u16 = ((nvm_data[1] as u16) << 8) | (nvm_data[0] as u16);
    // 0x33 (LSB) & 0x34 (MSB) -> u16
    let nvm_par_t2: u16 = ((nvm_data[3] as u16) << 8) | (nvm_data[2] as u16);
    // 0x35 -> i8 (Note: This is an 8-bit signed value!)
    let nvm_par_t3: i8 = nvm_data[4] as i8;
    let par_t1 = (nvm_par_t1 as f32) / 0.00390625; // 2^-8
    let par_t2 = (nvm_par_t2 as f32) / 1073741824.0; // 2^30
    let par_t3 = (nvm_par_t3 as f32) / 281474976710656.0; // 2^48
    loop {
        Timer::after_secs(2).await;
        let raw_temp = read_temp(&mut i2c).await;
        let partial_data1 = (raw_temp as f32) - par_t1;
        let partial_data2 = partial_data1 * par_t2;

        // t_lin is the compensated temperature in degrees Celsius
        let t_lin = partial_data2 + (partial_data1 * partial_data1) * par_t3;
        let t_int = t_lin as u8;
        if t_int > max {
            max = t_int;
            i2c.write(EEPROM_ADDR, &[0x00, 0x00, max]).await.unwrap();
        } else if t_int < min {
            min = t_int;
            i2c.write(EEPROM_ADDR, &[0x00, 0x01, min]).await.unwrap();
        }
        info!("current: {} , max: {} , min: {}", t_int, max, min);
    }
}