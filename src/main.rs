#![no_main]
#![no_std]

use panic_halt as _; // panic handler

use stm32f1xx_hal as hal;

use crate::{
    button::{Active, ButtonEvent, Debouncer, LongPressButton},
    serial::{Command, Report, SerialProtocol},
};
use cortex_m_rt::entry;
use embedded_hal::{digital::v2::OutputPin, Direction as RotaryDirection};
use hal::{
    pac,
    prelude::*,
    qei::QeiOptions,
    timer::{Tim2NoRemap, Timer},
};

mod button;
mod serial;

#[entry]
fn main() -> ! {
    let mut cp =
        cortex_m::peripheral::Peripherals::take().expect("failed to get cortex_m peripherals");
    let dp = pac::Peripherals::take().expect("failed to get stm32 peripherals");

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs.
    // RCC = Reset and Clock Control
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // Needed in order for MonoTimer to work properly
    cp.DCB.enable_trace();

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

    let usart_pins = (gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh), gpioa.pa10);
    let mut protocol = SerialProtocol::new(dp.USART1, usart_pins, &mut afio, &mut rcc.apb2, clocks);

    // PWM Setup
    let pwm_pin = gpioa.pa8.into_alternate_push_pull(&mut gpioa.crh);
    let mut pwm =
        Timer::tim1(dp.TIM1, &clocks, &mut rcc.apb2).pwm(pwm_pin, &mut afio.mapr, 1.khz()).split();

    pwm.set_duty(pwm.get_max_duty());

    // Connect a rotary encoder to pins A0 and A1.
    let rotary_encoder_pins = (gpioa.pa0, gpioa.pa1);
    // Tim2NoRemap relates to how you can "remap" pins used on timer 2 for certain peripherals.
    // https://docs.rs/stm32f1xx-hal/0.6.1/stm32f1xx_hal/timer/index.html
    let rotary_encoder = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).qei::<Tim2NoRemap, _>(
        rotary_encoder_pins,
        &mut afio.mapr,
        QeiOptions::default(),
    );

    let button_pin = gpioa.pa3.into_pull_up_input(&mut gpioa.crl);
    let debounced_encoder_pin = Debouncer::new(button_pin, Active::Low, 30, 100);
    let mut long_press_button = LongPressButton::new(debounced_encoder_pin, 1000, cp.DWT, clocks);

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

            current_count = new_count;
            protocol.report(Report::DialValue { diff: diff as i8 }).unwrap();
        }

        match long_press_button.poll() {
            Some(ButtonEvent::Pressed) => {
                led.set_low().unwrap();
            },
            Some(ButtonEvent::ShortRelease) => {
                protocol.report(Report::Click).unwrap();
                led.set_high().unwrap();
            },
            Some(ButtonEvent::LongPress) => {
                led.set_high().unwrap();
            },
            Some(ButtonEvent::LongRelease) => {},
            _ => {},
        }

        match protocol.poll().unwrap() {
            Some(Command::Brightness { value }) => {
                let adjusted = (value as f32 / u16::MAX as f32) * pwm.get_max_duty() as f32;
                pwm.set_duty(adjusted as u16);
            },
            _ => {},
        }

        delay.delay_ms(10_u32);
    }
}
