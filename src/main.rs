#![no_main]
#![no_std]

use panic_halt as _; // panic handler

use stm32f1xx_hal as hal;

use core::fmt::Write;
use cortex_m_rt::entry;
use embedded_hal::{digital::v2::OutputPin, Direction as RotaryDirection};
use hal::{
    pac,
    prelude::*,
    qei::QeiOptions,
    serial::Config as SerialConfig,
    timer::{Tim2NoRemap, Timer},
};

mod protocol;

#[entry]
fn main() -> ! {
    let cp = cortex_m::peripheral::Peripherals::take().expect("failed to get cortex_m peripherals");
    let dp = pac::Peripherals::take().expect("failed to get stm32 peripherals");

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs.
    // RCC = Reset and Clock Control
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // Prepare the alternate function I/O registers
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    // Grab the GPIO banks we'll use.
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);

    // Set up the LED (B12).
    let mut led = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);

    // Create a delay abstraction based on SysTick.
    let mut delay = hal::delay::Delay::new(cp.SYST, clocks);

    // Set up serial communication on pins A9 (Tx) and A10 (Rx), with 115200 baud.
    let serial_config = SerialConfig::default().baudrate(115200.bps());
    let usart1 = dp.USART1;
    let usart_pins = (gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh), gpioa.pa10);
    let mapr = &mut afio.mapr;
    let apb = &mut rcc.apb2;
    let usart = hal::serial::Serial::usart1(usart1, usart_pins, mapr, serial_config, clocks, apb);
    let (mut tx, _rx) = usart.split();

    // PWM Setup
    let pwm_pin = gpioa.pa8.into_alternate_af1();
    let pwm_timer = dp.TIM1;
    let mut pwm = hal::pwm::tim1(pwm_timer, pwm_pin, clocks, 1.khz());
    let max_duty = pwm.get_max_duty();

    writeln!(tx, "Max duty: {:?}\r", max_duty).unwrap();

    let mut current_duty = max_duty / 4;
    pwm.set_duty(current_duty);
    pwm.enable();

    // Connect a rotary encoder to pins A0 and A1.
    let rotary_encoder_pins = (gpioa.pa0, gpioa.pa1);
    // Tim2NoRemap relates to how you can "remap" pins used on timer 2 for certain peripherals.
    // https://docs.rs/stm32f1xx-hal/0.6.1/stm32f1xx_hal/timer/index.html
    let rotary_encoder = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).qei::<Tim2NoRemap, _>(
        rotary_encoder_pins,
        &mut afio.mapr,
        QeiOptions::default(),
    );

    let mut current_count = rotary_encoder.count();

    loop {
        let new_count = rotary_encoder.count();

        if new_count != current_count {
            let current_direction = rotary_encoder.direction();
            let diff = new_count.wrapping_sub(current_count) as i16;

            match current_direction {
                RotaryDirection::Upcounting => {
                    led.set_low().unwrap();
                },
                RotaryDirection::Downcounting => {
                    led.set_high().unwrap();
                },
            }

            if diff > 0 {
                current_duty = current_duty.saturating_add((diff * 100) as u16);
                if current_duty > max_duty {
                    current_duty = max_duty;
                }
            } else {
                current_duty = current_duty.saturating_sub((-diff * 100) as u16);
            }

            writeln!(tx, "Current duty: {:?}\r", current_duty).unwrap();

            pwm.set_duty(current_duty);

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
