#![no_main]
#![no_std]

use crate::rgb_led::LED_COUNT;
use panel_protocol::PulseMode;
use panic_reset as _; // panic handler

use stm32f4xx_hal as hal;

use crate::{
    button::{Active, Button, ButtonEvent, Debouncer},
    counter::Counter,
    overhead_light::OverheadLight,
    rgb_led::{LedStrip, Pulser, Rgb},
    serial::{Command, Report, SerialProtocol},
};
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use hal::{
    otg_fs::{UsbBus, USB},
    prelude::*,
    pwm,
    qei::Qei,
    spi::{Mode as SpiMode, NoMiso, NoSck, Phase, Polarity, Spi},
    stm32,
    timer::MonoTimer,
};
use usb_device::device::{UsbDeviceBuilder, UsbVidPid};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

mod bootload;
mod button;
mod counter;
mod overhead_light;
mod rgb_led;
mod serial;

static mut USB_ENDPOINT_MEMORY: [u32; 1024] = [0; 1024];

#[entry]
fn main() -> ! {
    let panel_serial_number = env!("PANEL_SERIAL_NUMBER");
    let cp = cortex_m::peripheral::Peripherals::take().expect("failed to get cortex_m peripherals");
    let dp = stm32::Peripherals::take().expect("failed to get stm32 peripherals");

    // This call needs to happen as early as possible in the firmware.
    bootload::jump_to_bootloader_if_requested(&dp);

    // Take ownership over the raw devices and convert them into the corresponding
    // HAL structs.
    // RCC = Reset and Clock Control
    let rcc = dp.RCC.constrain();

    // The various system clocks need to be configured to particular values
    // to work with USB - we'll set them up here.
    let clocks = rcc
        .cfgr
        .use_hse(25.mhz()) // Use the High Speed External 25MHz crystal
        .sysclk(48.mhz()) // The main system clock will be 48MHz
        .require_pll48clk()
        .freeze();

    // TODO(bschwind) - Find or write an equivalent for this in stm32f4xx-hal
    // assert!(clocks.usbclk_valid());

    // Grab the GPIO banks we'll use.
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    // Set up the LED (C13).
    let mut led = gpioc.pc13.into_push_pull_output();
    led.set_high().unwrap();

    // SPI Setup (for WS8212b RGB LEDs)
    let mosi_pin = gpiob.pb15.into_alternate_af5();
    let spi_pins = (NoSck, NoMiso, mosi_pin);
    let spi_mode = SpiMode { polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition };

    let spi = Spi::spi2(dp.SPI2, spi_pins, spi_mode, 2250.khz().into(), clocks);

    let mut led_strip = LedStrip::new(spi);

    let timer = MonoTimer::new(cp.DWT, cp.DCB, clocks);
    // Human relaxed breath time: around 4s in/out and 4s wait
    let mut pulser = Pulser::new(4000, &timer);

    // PWM Setup
    let pwm_freq = 1.khz();

    let back_light_pwm_pins = (
        gpioa.pa0.into_alternate_af2(),
        gpioa.pa1.into_alternate_af2(),
        gpioa.pa2.into_alternate_af2(),
        gpioa.pa3.into_alternate_af2(),
    );

    let front_light_pwm_pins = (
        gpioa.pa6.into_alternate_af2(),
        gpioa.pa7.into_alternate_af2(),
        gpiob.pb0.into_alternate_af2(),
        gpiob.pb1.into_alternate_af2(),
    );

    let (pwm1, pwm2, pwm3, pwm4) = pwm::tim5(dp.TIM5, back_light_pwm_pins, clocks, pwm_freq);
    let (pwm5, pwm6, pwm7, pwm8) = pwm::tim3(dp.TIM3, front_light_pwm_pins, clocks, pwm_freq);

    // The overhead light closer to the screen.
    let mut front_light = OverheadLight::new(pwm1, pwm2, pwm3, pwm4);

    // The overhead light farther away from the screen.
    let mut back_light = OverheadLight::new(pwm5, pwm6, pwm7, pwm8);

    // Connect a rotary encoder to pins A8 and A9.
    let rotary_encoder_timer = dp.TIM1;
    let rotary_encoder_pins = (gpioa.pa8.into_alternate_af1(), gpioa.pa9.into_alternate_af1());
    let rotary_encoder = Qei::new(rotary_encoder_timer, rotary_encoder_pins);

    let mut counter = Counter::new(rotary_encoder);
    // Previous intensity when used with dial turn intensity
    let mut quadrature_zero_value = 0.0;
    let mut previous_dial_turn_intensity = 0.0;

    let button_pin = gpioa.pa10.into_pull_up_input();
    let debounced_encoder_pin = Debouncer::new(button_pin, Active::Low, 30, 3000);
    let mut encoder_button = Button::new(debounced_encoder_pin);

    let mut led_color = (0u8, 30u8, 255u8);
    let mut led_pulse = PulseMode::Solid;

    // Set up USB communication.
    // First we set the D+ pin low for 100ms to simulate a USB
    // reset condition. This ensures more stable operation when
    // booting up after a USB DFU firmware update. Without this,
    // the USB serial device sometimes doesn't show on the host OS
    // after booting up.
    let mut delay = hal::delay::Delay::new(cp.SYST, clocks);
    let mut usb_pin_d_plus = gpioa.pa12.into_push_pull_output();
    usb_pin_d_plus.set_low().unwrap();
    delay.delay_ms(100_u32);

    // Now we can connect as a USB serial device to the host.
    let usb_pin_d_plus = usb_pin_d_plus.into_alternate_af10();
    let usb_pin_d_minus = gpioa.pa11.into_alternate_af10();

    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        hclk: clocks.hclk(),

        pin_dm: usb_pin_d_minus,
        pin_dp: usb_pin_d_plus,
    };

    let usb_bus = UsbBus::new(usb, unsafe { &mut USB_ENDPOINT_MEMORY });
    let serial = SerialPort::new(&usb_bus);

    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("tonari")
        .product("panel_controller")
        .serial_number(panel_serial_number)
        .device_class(USB_CLASS_CDC)
        .build();

    let mut protocol = SerialProtocol::new(usb_dev, serial);

    // Turn the LED on to indicate we've powered up successfully.
    led.set_low().unwrap();

    let mut current_led = 0usize;
    let mut led_intensities = [0.0; LED_COUNT];

    loop {
        match encoder_button.poll() {
            Some(ButtonEvent::Press) => {
                protocol.report(Report::Press).unwrap();
                led.set_low().unwrap();
            },
            Some(ButtonEvent::Release) => {
                protocol.report(Report::Release).unwrap();
                led.set_high().unwrap();
            },
            _ => {},
        }

        if let Some(diff) = counter.poll() {
            if !encoder_button.is_pressed() {
                protocol.report(Report::DialValue { diff }).unwrap();

                current_led = current_led.wrapping_add(diff as usize) % 4;
            }
        }

        // TODO(bschwind) - Report any poll errors back to the USB host if possible.
        for command in protocol.poll().unwrap() {
            match command {
                Command::Brightness { target, value } => match target {
                    0 => front_light.set_brightness(value),
                    1 => back_light.set_brightness(value),
                    _ => {},
                },
                Command::Temperature { target, value } => match target {
                    0 => front_light.set_color_temperature(value),
                    1 => back_light.set_color_temperature(value),
                    _ => {},
                },
                Command::Led { r, g, b, pulse_mode } => {
                    led_color = (r, g, b);
                    led_pulse = pulse_mode;
                },
                Command::Bootload => {
                    led.set_high().unwrap();
                    bootload::request_bootloader();
                },
                _ => {},
            }
        }

        match led_pulse {
            PulseMode::Breathing { interval_ms } => {
                pulser.set_interval_ms(u16::from(interval_ms) as u32, &timer);
                let intensity = pulser.intensity();

                led_strip.set_all(Rgb::new(
                    (led_color.0 as f32 * intensity) as u8,
                    (led_color.1 as f32 * intensity) as u8,
                    (led_color.2 as f32 * intensity) as u8,
                ));
            },
            PulseMode::DialTurn => {
                let mut new_led_intensities = [0.0; LED_COUNT];
                new_led_intensities[current_led] = 1.0;
                let mut leds = [Rgb::new(led_color.0, led_color.1, led_color.2); LED_COUNT];
                for (led_intensity, new_led_intensity) in
                    led_intensities.iter_mut().zip(new_led_intensities.iter())
                {
                    *led_intensity = 0.995 * *led_intensity + 0.005 * new_led_intensity;
                }

                for (mut led, intensity) in leds.iter_mut().zip(led_intensities.iter()) {
                    led.r = ((led.r as f32) * intensity) as u8;
                    led.g = ((led.g as f32) * intensity) as u8;
                    led.b = ((led.b as f32) * intensity) as u8;
                }

                led_strip.set_colors(&leds);
            },
            PulseMode::Solid => {
                let intensity = 1.0;

                led_strip.set_all(Rgb::new(
                    (led_color.0 as f32 * intensity) as u8,
                    (led_color.1 as f32 * intensity) as u8,
                    (led_color.2 as f32 * intensity) as u8,
                ));
            },
        };
    }
}
