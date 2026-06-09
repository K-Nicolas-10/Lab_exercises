
#[embassy_executor::task]
async fn adress_task(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
    mut s1: ExtiInput<'static>,
) {
    let mut current_adress: u8 = 0x08;
    loop {
        let p = i2c.write_read(current_adress, &[0x00], &mut [0u8; 1]).await;
        match p {
            Ok(_) => info!("I2C Device found at adress:  {=u8:#04x}", current_adress),
            Err(_) => info!("No device found at address: {=u8:#04x}", current_adress),
        }
        s1.wait_for_falling_edge().await;
        current_adress += 1;
        if current_adress == 0x78 {
            current_adress = 0x08;
        }
    }
}