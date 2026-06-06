#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_stm32::interrupt::EXTI6;
use embassy_stm32::sai::word::U4;
use embassy_stm32::{adc::Adc, exti::ExtiInput, gpio::*, timer::low_level::Timer};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use panic_probe as _;

#[derive(Clone, Copy)]
enum GateMode {
    Xor,
    Or,
    And,
}

enum GateState {
    On,
    Default,
}

static STATE: Signal<CriticalSectionRawMutex, GateState> = Signal::new();
static MODE: GateMode = GateMode::Xor;
#[embassy_executor::task]
async fn assess_gate(mut s1: ExtiInput<'static>, mut s2: ExtiInput<'static>) {
    loop {
        let gate_out = match MODE {
            GateMode::Xor => s1.is_low() ^ s2.is_low(),
            GateMode::And => s1.is_low() && s2.is_low(),
            GateMode::Or => s1.is_low() || s2.is_low(),
        };
        match gate_out {
            true => STATE.signal(GateState::On),
            false => STATE.signal(GateState::Default),
        }
        select(s1.wait_for_any_edge(), s2.wait_for_any_edge()).await;
    }
}

#[embassy_executor::task]
async fn led_task(mut blue: Output<'static>) {
    loop {
        let p = STATE.wait().await;
        match p {
            GateState::Default => {
                blue.set_low();
            }
            GateState::On => {
                blue.set_high();
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut green = Output::new(p.PB6, Level::Low, Speed::Medium);
    let mut red = Output::new(p.PB7, Level::Low, Speed::Medium);
    let mut blue = Output::new(p.PA5, Level::Low, Speed::Medium);
    let mut yellow = Output::new(p.PC9, Level::Low, Speed::Medium);
    let mut s1 = ExtiInput::new(p.PA6, p.EXTI6, Pull::Up); //margine
    let mut s2 = ExtiInput::new(p.PA7, p.EXTI7, Pull::Up); //dreapta

    spawner.spawn(assess_gate(s1, s2)).unwrap();
    spawner.spawn(led_task(blue)).unwrap();
}

 
