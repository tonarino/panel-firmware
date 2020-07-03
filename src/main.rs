#![no_main]
#![no_std]

// Halt on panic
#[allow(unused_extern_crates)] // NOTE(allow) bug rust-lang/rust#53964
extern crate panic_halt; // panic handler

use core::fmt::Write;
use cortex_m;
use cortex_m_rt::entry;
use hal::{
    hal::Direction as RotaryDirection, prelude::*, serial::config::Config as SerialConfig, stm32,
};
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) =
        (stm32::Peripherals::take(), cortex_m::peripheral::Peripherals::take())
    {
        // Set up the LED. On the Nucleo-446RE it's connected to pin PA5.
        let gpioc = dp.GPIOC.split();
        let mut led = gpioc.pc13.into_push_pull_output();

        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Create a delay abstraction based on SysTick.
        let mut delay = hal::delay::Delay::new(cp.SYST, clocks);

        // Set up serial communication on pins A9 (Tx) and A10 (Rx).
        let serial_config = SerialConfig::default().baudrate(115200.bps());
        let usart1 = dp.USART1;

        let gpioa = dp.GPIOA.split();
        let usart_pins = (gpioa.pa9.into_alternate_af7(), gpioa.pa10.into_alternate_af7());
        let usart = hal::serial::Serial::usart1(usart1, usart_pins, serial_config, clocks).unwrap();
        let (mut tx, _rx) = usart.split();

        // Connect a rotary encoder to pins A0 and A1.
        let rotary_encoder_pins = (gpioa.pa0.into_alternate_af1(), gpioa.pa1.into_alternate_af1());
        let rotary_encoder_timer = dp.TIM2;
        let rotary_encoder = hal::qei::Qei::tim2(rotary_encoder_timer, rotary_encoder_pins);

        let mut current_count = rotary_encoder.count();

        loop {
            let new_count = rotary_encoder.count();

            if new_count != current_count {
                let current_direction = rotary_encoder.direction();

                let diff = match current_direction {
                    RotaryDirection::Upcounting => {
                        led.set_low().unwrap();
                        (new_count - current_count) as i32
                    },
                    RotaryDirection::Downcounting => {
                        led.set_high().unwrap();
                        -(current_count as i32 - new_count as i32)
                    },
                };

                current_count = new_count;
                writeln!(
                    tx,
                    "Diff: {}, Count: {}, Direction: {:?}\r",
                    diff, current_count, current_direction
                )
                .unwrap();
            }

            delay.delay_ms(10_u32);
        }
    }

    loop {}
}
