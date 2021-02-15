use stm32f4xx_hal as hal;

use embedded_hal::spi::FullDuplex;
use hal::timer::{Instant, MonoTimer};
use nb::block;

// Reference implementation:
// https://github.com/smart-leds-rs/ws2812-spi-rs/blob/fac281eb57b5f72c48e368682645e3b0bd5b4b83/src/lib.rs

const LED_COUNT: usize = 2;
const PI: f32 = 3.141_592_7e0;

pub struct LedStrip<F: FullDuplex<u8>> {
    spi_bus: F,
}

pub struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl<F: FullDuplex<u8>> LedStrip<F> {
    pub fn new(spi_bus: F) -> Self {
        Self { spi_bus }
    }

    pub fn set_all(&mut self, rgb: Rgb) {
        self.flush();

        for _led in 0..LED_COUNT {
            self.write_byte(rgb.g);
            self.write_byte(rgb.r);
            self.write_byte(rgb.b);
        }

        self.flush();
    }

    #[allow(unused)]
    pub fn set_colors(&mut self, rgb_data: &[Rgb; LED_COUNT]) {
        self.flush();

        for led in rgb_data {
            self.write_byte(led.g);
            self.write_byte(led.r);
            self.write_byte(led.b);
        }

        self.flush();
    }

    fn write_byte(&mut self, data: u8) {
        let mut data = data;
        let patterns = [0b1000_1000, 0b1000_1110, 0b11101000, 0b11101110];

        for _ in 0..4 {
            let bits = (data & 0b1100_0000) >> 6;
            let _ = block!({
                let _ = self.spi_bus.send(patterns[bits as usize]);
                self.spi_bus.read()
            });

            data <<= 2;
        }
    }

    fn flush(&mut self) {
        for _ in 0..20 {
            let _ = block!({
                let _ = self.spi_bus.send(0).map_err(|_| ());
                self.spi_bus.read()
            });
        }
    }
}

/// U64Instant::elapsed() tries to correct the u32 overflow of the underlying Instant. It is
/// supposed to be accurate as long as the function is called frequently enough i.e. at least
/// once per 1 minute 29 seconds.
struct U64Instant {
    elapsed: u64,
    last_elapsed_u32: u32,
    instant: Instant,
}

impl From<Instant> for U64Instant {
    fn from(instant: Instant) -> Self {
        let elapsed = instant.elapsed();

        Self { elapsed: elapsed as u64, last_elapsed_u32: elapsed, instant }
    }
}

impl U64Instant {
    fn elapsed(&mut self) -> u64 {
        let elapsed_u32 = self.instant.elapsed();
        let mut diff = elapsed_u32 as i64 - self.last_elapsed_u32 as i64;
        if diff < 0 {
            diff += u32::MAX as i64 + 1;
        }

        self.last_elapsed_u32 = elapsed_u32;
        self.elapsed += diff as u64;
        self.elapsed
    }
}

pub struct Pulser {
    instant: U64Instant,
    interval_ticks: f32,
}

impl Pulser {
    pub fn new(interval_ms: u32, timer: &MonoTimer) -> Self {
        let instant = timer.now().into();
        let interval_ticks = timer.frequency().0 as f32 * (interval_ms as f32 / 1000.0);

        Self { instant, interval_ticks }
    }

    pub fn intensity(&mut self) -> f32 {
        let intervals = self.instant.elapsed() as f32 / self.interval_ticks;
        let pulse = (libm::sinf(intervals) + 1.0) * 0.5;
        let skip_one = if libm::sinf((intervals + PI / 2.0) / 2.0) >= 0.0 { 1.0 } else { 0.0 };

        pulse * skip_one
    }
}
