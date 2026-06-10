#[embassy_executor::task]
async fn read_eeprom(
    mut i2c: I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
) {
    let mem_addr: u16 = 0x0000;
    let memory_address: [u8; 2] = mem_addr.to_be_bytes();
    let mut data: [u8; 2] = [0, 0];
    i2c.write_read(EEPROM_ADDR, &memory_address, &mut data)
        .await
        .unwrap();
    let mut boot_number: u16 = u16::from_be_bytes(data);
    if boot_number == 0xFFFF {
        boot_number = 0;
    }
    boot_number = boot_number.wrapping_add(1);
    let boot_bytes = boot_number.to_be_bytes();
    let write_buf = [
        memory_address[0],
        memory_address[1],
        boot_bytes[0],
        boot_bytes[1],
    ];
    i2c.write(EEPROM_ADDR, &write_buf).await.unwrap();
    Timer::after_millis(5).await;
    info!("boot #: {}", boot_number);
}
