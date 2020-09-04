use embedded_hal::PwmPin;

pub struct OverheadLight<P1, P2, P3, P4>
where
    P1: PwmPin<Duty = u16>,
    P1: PwmPin<Duty = u16>,
    P3: PwmPin<Duty = u16>,
    P4: PwmPin<Duty = u16>,
{
    brightness_c1: P1,
    brightness_c2: P2,
    color_c1: P3,
    color_c2: P4,
}

impl<P1, P2, P3, P4> OverheadLight<P1, P2, P3, P4>
where
    P1: PwmPin<Duty = u16>,
    P2: PwmPin<Duty = u16>,
    P3: PwmPin<Duty = u16>,
    P4: PwmPin<Duty = u16>,
{
    pub fn new(
        mut brightness_c1: P1,
        mut brightness_c2: P2,
        mut color_c1: P3,
        mut color_c2: P4,
    ) -> Self {
        brightness_c1.enable();
        brightness_c2.enable();
        color_c1.enable();
        color_c2.enable();

        // Set maximum brightness
        brightness_c1.set_duty(0);
        brightness_c2.set_duty(0);

        // Set white color temperature
        color_c1.set_duty(0);
        color_c2.set_duty(0);

        OverheadLight { brightness_c1, brightness_c2, color_c1, color_c2 }
    }

    /// Sets the brightness of both channels.
    /// 0 = Off
    /// u16::MAX = Full brightness
    pub fn set_brightness(&mut self, brightness: u16) {
        // Invert the value because our transistor circuit inverts the PWM signal.
        let brightness = u16::MAX - brightness;

        let adjusted = ((brightness as f32 / u16::MAX as f32)
            * self.brightness_c1.get_max_duty() as f32) as u16;
        self.brightness_c1.set_duty(adjusted);
        self.brightness_c2.set_duty(adjusted);
    }

    /// Sets the color temperature of both channels.
    /// 0 = Full yellow
    /// u16::MAX = Full white
    pub fn set_color_temperature(&mut self, color: u16) {
        // Invert the value because our transistor circuit inverts the PWM signal.
        let color = u16::MAX - color;

        let adjusted =
            ((color as f32 / u16::MAX as f32) * self.color_c1.get_max_duty() as f32) as u16;
        self.color_c1.set_duty(adjusted);
        self.color_c2.set_duty(adjusted);
    }
}
