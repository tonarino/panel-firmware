#![no_main]
#![no_std]

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

mod button;
mod counter;
mod overhead_light;
mod rgb_led;
mod serial;

static mut USB_ENDPOINT_MEMORY: [u32; 1024] = [0; 1024];

#[entry]
fn main() -> ! {
    let cp = cortex_m::peripheral::Peripherals::take().expect("failed to get cortex_m peripherals");
    let dp = stm32::Peripherals::take().expect("failed to get stm32 peripherals");

    // This call needs to happen as early as possible in the firmware.
    jump_to_bootloader_if_requested(&dp);

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

    // Set up USB communications
    let usb_pin_d_plus = gpioa.pa12.into_alternate_af10();
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
        .product("tonari dashboard controller")
        .serial_number("tonari-dashboard-controller-v1")
        .device_class(USB_CLASS_CDC)
        .build();

    let mut protocol = SerialProtocol::new(usb_dev, serial);

    // SPI Setup (for WS8212b RGB LEDs)
    let mosi_pin = gpiob.pb15.into_alternate_af5();
    let spi_pins = (NoSck, NoMiso, mosi_pin);
    let spi_mode = SpiMode { polarity: Polarity::IdleLow, phase: Phase::CaptureOnFirstTransition };

    let spi = Spi::spi2(dp.SPI2, spi_pins, spi_mode, 2250.khz().into(), clocks);

    let mut led_strip = LedStrip::new(spi);

    let timer = MonoTimer::new(cp.DWT, cp.DCB, clocks);
    let mut pulser = Pulser::new(700, &timer);

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

    let button_pin = gpioa.pa10.into_pull_up_input();
    let debounced_encoder_pin = Debouncer::new(button_pin, Active::Low, 30, 3000);
    let mut encoder_button = Button::new(debounced_encoder_pin);

    let mut led_color = (0u8, 30u8, 255u8);
    let mut led_pulse = false;

    // Turn the LED on to indicate we've powered up successfully.
    led.set_low().unwrap();

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
                Command::Led { r, g, b, pulse } => {
                    led_color = (r, g, b);
                    led_pulse = pulse;
                },
                Command::Bootload => {
                    led.set_high().unwrap();
                    request_bootloader();
                },
                _ => {},
            }
        }

        let intensity = if led_pulse { pulser.intensity() } else { 1.0 };
        led_strip.set_all(Rgb::new(
            (led_color.0 as f32 * intensity) as u8,
            (led_color.1 as f32 * intensity) as u8,
            (led_color.2 as f32 * intensity) as u8,
        ));
    }
}

const MAGIC_BOOTLOADER_NUMBER: u32 = 131981;

fn request_bootloader() -> ! {
    let dp = unsafe { stm32::Peripherals::steal() };

    enable_backup_domain(&dp);

    let rtc = &dp.RTC;
    let backup_register = &rtc.bkpr;

    backup_register[0].write(|w| {
        w.bkp().bits(MAGIC_BOOTLOADER_NUMBER)
    });

    cortex_m::asm::dsb();

    disable_backup_domain(&dp);

    cortex_m::peripheral::SCB::sys_reset();
}

fn enable_backup_domain(dp: &hal::stm32::Peripherals) {
    let rtc = &dp.RTC;
    let pwr = &dp.PWR;
    let rcc = &dp.RCC;

    // Enable the power interface clock by setting the PWREN bits in the RCC_APB1ENR register
    rcc.apb1enr.write(|w| {
        w.pwren().bit(true)
    });

    cortex_m::asm::dsb();

    // Set the DBP bit in the Section 5.4.1 to enable access to the backup domain
    pwr.cr.write(|w| {
        w.dbp().bit(true)
    });

    // Select the RTC clock source
    // if rcc.bdcr.read().lserdy().bit_is_clear() {
    //     enable_lse(rcc, bypass);
    // }

    // Enable the RTC clock by programming the RTCEN [15] bit in the Section 7.3.20: RCC Backup domain control register (RCC_BDCR)
    rcc.bdcr.write(|w| {
        w.rtcen().bit(true)
    });

    // Disable write protect?
    rtc.wpr.write(|w| {
        unsafe { w.bits(0xCA) }
    });

    cortex_m::asm::dsb();

    rtc.wpr.write(|w| {
        unsafe { w.bits(0x53) }
    });
}

fn disable_backup_domain(dp: &stm32::Peripherals) {
    let pwr = &dp.PWR;

    pwr.cr.write(|w| {
        w.dbp().bit(false)
    });
}

fn jump_to_bootloader_if_requested(dp: &stm32::Peripherals) {
    let rtc = &dp.RTC;
    let backup_register = &rtc.bkpr;

    let magic_num: u32 = backup_register[0].read().bkp().bits();

    if magic_num == MAGIC_BOOTLOADER_NUMBER {
        enable_backup_domain(&dp);

        backup_register[0].write(|w| {
            w.bkp().bits(0u32)
        });

        disable_backup_domain(&dp);

        unsafe {
            cortex_m::asm::bootload(0x1FFF0000 as *const u32);
        }
    }
}

// fn bootloader() -> ! {
//     // cortex_m::interrupt::disable();

//     unsafe {
//         let mut cp = cortex_m::Peripherals::steal();
//         let dp = stm32::Peripherals::steal();

//         // // Deinit the RCC
//         let rcc = dp.RCC.constrain();
//         let _clocks = rcc
//             .cfgr
//             .freeze();

//         cortex_m::asm::dsb();

//         // // Reset the SysTick timer
//         cp.SYST.clear_current();

//         cortex_m::asm::dsb();

//         let rtc = dp.RTC;
//         let backup_register = &rtc.bkpr;
//         // 20 (32-bit) backup registers used to store 80 bytes
//         // of user application data when VDD power is not present.
//         backup_register[0].write(|w| {
//             w.bkp().bits(MAGIC_BOOTLOADER_NUMBER)
//         });
//         // stm32f4xx_hal::stm32::rtc::BKPR.write();

//         // // Disable interrupts
//         // cortex_m::interrupt::disable();
//         // cortex_m::asm::bootload(0x0010_0000 as *const u32);
//         cortex_m::asm::bootload(0x1FFF0000 as *const u32);

//         // cortex_m::asm::dsb();

//         // 0x1FFF76DE - Bootloader memory location
//         // 12 Kbyte starting from address 0x20000000
//         // are used by the bootloader firmware for RAM
//         // 29 Kbyte starting from address 0x1FFF0000,
//         // contain the bootloader firmware
//         // 0x20003000 - 0x2001FFFF: RAM
//         // 0x1FFF0000 - 0x1FFF77FF: System Memory


//         // cortex_m::asm::bootload(0x0 as *const u32);
//         // cortex_m::asm::bootload(0x1FFF0000 as *const u32);
//         // cortex_m::asm::bootload(0x08000000 as *const u32);
//         // cortex_m::asm::bootstrap(0x20003000 as *const u32, 0x1FFF76DE as *const u32);
//         // cortex_m::asm::bootstrap(0x1FFF0000 as *const u32, (0x1FFF0000 + 4) as *const u32);
//     }

//     // cortex_m::asm::bootload(0x00000004 as *const u32);
//     // cortex_m::asm::bootload(0x08000000 as *const u32);
//     // cortex_m::asm::bootload(0x0 as *const u32);
//     // cortex_m::asm::bootload(0x1FFF76DE as *const u32);

    
//     // stm32f4xx_hal::pac::SCB::sys_reset();
//     // unsafe {
//     //     cortex_m::asm::bootload(0x1FFF0000 as *const u32);
//     // }
// }