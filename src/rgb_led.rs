use embedded_hal::spi::FullDuplex;
use nb::block;
use stm32f1xx_hal::time::{Instant, MonoTimer};

// Reference implementation:
// https://github.com/smart-leds-rs/ws2812-spi-rs/blob/fac281eb57b5f72c48e368682645e3b0bd5b4b83/src/lib.rs

const LED_COUNT: usize = 2;

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

#[allow(dead_code)]
pub struct Pulser {
    instant: Instant,
    interval_ticks: f32,
}

impl Pulser {
    pub fn new(interval_ms: u32, timer: &MonoTimer) -> Self {
        let instant = timer.now();
        let interval_ticks = timer.frequency().0 as f32 * (interval_ms as f32 / 1000.0);

        Self { instant, interval_ticks }
    }

    #[allow(dead_code)]
    pub fn intensity(&self) -> f32 {
        let intervals = self.instant.elapsed() as f32 / self.interval_ticks;
        (libm::sinf(intervals) + 1.0) / 2.0
    }
}
