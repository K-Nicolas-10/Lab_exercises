static COORDS: Signal<CriticalSectionRawMutex, (i16, i16)> = Signal::new();

#[embassy_executor::task]
async fn blink_led(
    mut red: Output<'static>,
    mut green: Output<'static>,
    mut blue: Output<'static>,
    mut yellow: Output<'static>,
) {
    let thresh_x = 940 as i16;
    let thresh_y = 200 as i16;
    let (mut x, mut y) = COORDS.wait().await;
    loop {
        match COORDS.try_take() {
            None => {}
            Some((a, b)) => (x, y) = (a, b),
        }
        if x > thresh_x {
            red.set_low();
            green.toggle();
            Timer::after_millis(10000 / x as u64).await;
        } else if x < thresh_x {
            green.set_low();
            red.toggle();
            Timer::after_millis((10000 / x as u64)).await;
        }
        if y > thresh_y {
            yellow.set_low();
            blue.toggle();
            Timer::after_millis(10000 / y as u64).await;
        } else if y < thresh_y {
            blue.set_low();
            yellow.toggle();
            Timer::after_millis(10000 / y as u64).await;
        }
    }
}

#[embassy_executor::task]
async fn util_fn(
    mut spi: embassy_stm32::spi::Spi<'static, embassy_stm32::mode::Async>,
    mut cs: Output<'static>,
) {
    const ACCEL_CONFIG: u8 = 0x1C;
    const ACCEL_OUT: u8 = 0x3B;
    {
        let mut tx = [0u8; 2];
        let mut rx = [0u8; 2];
        tx[0] = ACCEL_CONFIG;
        tx[1] = 0b0001_0000;
        cs.set_low();
        spi.transfer(&mut rx, &tx).await.unwrap();
        cs.set_high();
    }

    loop {
        let tx = [ACCEL_OUT | (1 << 7), 0, 0, 0, 0];
        let mut rx = [0u8; 5];
        cs.set_low();
        spi.transfer(&mut rx, &tx).await.unwrap();
        cs.set_high();
        Timer::after_millis(40).await;
        let x_raw = i16::from_be_bytes([rx[1], rx[2]]);
        let y_raw = i16::from_be_bytes([rx[3], rx[4]]);
        info!("x: {}, y: {}", x_raw, y_raw);
        COORDS.signal((x_raw, y_raw));
    }
}