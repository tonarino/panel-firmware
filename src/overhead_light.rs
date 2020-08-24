use embedded_hal::PwmPin;

pub struct OverheadLight<P1: PwmPin, P2: PwmPin, P3: PwmPin, P4: PwmPin> {
    brightness_c1: P1,
    brightness_c2: P2,
    color_c1: P3,
    color_c2: P4,
}

impl<
        P1: PwmPin<Duty = u16>,
        P2: PwmPin<Duty = u16>,
        P3: PwmPin<Duty = u16>,
        P4: PwmPin<Duty = u16>,
    > OverheadLight<P1, P2, P3, P4>
{
    pub fn new(mut c1: P1, mut c2: P2, mut c3: P3, mut c4: P4) -> Self {
        c1.enable();
        c2.enable();
        c3.enable();
        c4.enable();

        c1.set_duty(0);
        c2.set_duty(0);
        c3.set_duty(0);
        c4.set_duty(0);

        OverheadLight { brightness_c1: c1, brightness_c2: c2, color_c1: c3, color_c2: c4 }
    }

    pub fn set_brightness(&mut self, brightness: u16) {
        let adjusted = ((brightness as f64 / u16::MAX as f64)
            * self.brightness_c1.get_max_duty() as f64) as u16;
        self.brightness_c1.set_duty(adjusted);
        self.brightness_c2.set_duty(adjusted);
    }

    pub fn set_color_temperature(&mut self, color: u16) {
        let adjusted =
            ((color as f64 / u16::MAX as f64) * self.color_c1.get_max_duty() as f64) as u16;
        self.color_c1.set_duty(adjusted);
        self.color_c2.set_duty(adjusted);
    }
}
