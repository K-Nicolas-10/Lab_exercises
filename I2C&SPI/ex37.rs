#[embassy_executor::task]
async fn util_fn(
    mut spi: embassy_stm32::spi::Spi<'static, embassy_stm32::mode::Async>,
    mut cs: Output<'static>,
) {
    const ACCEL_CONFIG: u8 = 0x1C;
    let mut tx = [0u8; 11];
    let mut rx = [0u8; 11];
    tx[0] = ACCEL_CONFIG | (1 << 7);
    cs.set_low();
    spi.transfer(&mut rx, &tx).await.unwrap();
    cs.set_high();
    let mut address = ACCEL_CONFIG;
    for i in 0..=9 {
        let val = rx[i + 1];
        info!("Reg {=u8:#X}: {=u8:#X}", address, val);
        address += 1;
    }
}
