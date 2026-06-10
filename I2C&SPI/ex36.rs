#[embassy_executor::task]
async fn h(
    mut spi: embassy_stm32::spi::Spi<'static, embassy_stm32::mode::Async>,
    mut cs: Output<'static>,
) {
    let tx = [0x6B, 0x00];
    let mut rx = [0u8; 2];

    cs.set_low();
    spi.transfer(&mut rx, &tx).await.unwrap();
    cs.set_high();
    cs.set_low();
    let mut rx_buf = [0u8; 2];
    let tx_buf = [0x75 | (1 << 7), 0x00];
    spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
    cs.set_high();
    let id = rx_buf[1];
    info!("id: {}", id);
    let accl_config = [0x1C, 0b0000_1000];
    cs.set_low();
    spi.transfer(&mut rx_buf, &accl_config).await.unwrap();
    cs.set_high();
    loop {
        let mut rx_buf = [0u8; 7];

        let tx_buf = [0x3B | (1 << 7), 0, 0, 0, 0, 0, 0];
        cs.set_low();
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        cs.set_high();
        let x = i16::from_be_bytes([rx_buf[1], rx_buf[2]]);
        let y = i16::from_be_bytes([rx_buf[3], rx_buf[4]]);
        let z = i16::from_be_bytes([rx_buf[5], rx_buf[6]]);
        let ax = x as f32;
        let ay = y as f32;
        let az = z as f32;

        let mag = libm::sqrtf(ax * ax + ay * ay + az * az);

        if mag < 0.2 * 8192. || mag > 2.5 * 8192. {
            info!("Fault: mag: {}", mag);
        }
    }
}