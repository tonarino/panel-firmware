use embedded_hal::PwmPin;

pub struct FanController<P1, P2, P3, P4>
where
    P1: PwmPin<Duty = u16>,
    P1: PwmPin<Duty = u16>,
    P3: PwmPin<Duty = u16>,
    P4: PwmPin<Duty = u16>,
{
    fan_1: P1,
    fan_2: P2,
    fan_3: P3,
    fan_4: P4,
}

impl<P1, P2, P3, P4> FanController<P1, P2, P3, P4>
where
    P1: PwmPin<Duty = u16>,
    P2: PwmPin<Duty = u16>,
    P3: PwmPin<Duty = u16>,
    P4: PwmPin<Duty = u16>,
{
    pub fn new(mut fan_1: P1, mut fan_2: P2, mut fan_3: P3, mut fan_4: P4) -> Self {
        fan_1.enable();
        fan_2.enable();
        fan_3.enable();
        fan_4.enable();

        fan_1.set_duty(0);
        fan_2.set_duty(0);
        fan_3.set_duty(0);
        fan_4.set_duty(0);

        Self { fan_1, fan_2, fan_3, fan_4 }
    }

    /// 0 = Off
    /// u16::MAX = Maximum Speed
    pub fn set_speed(&mut self, speed: u16, fan_index: u8) {
        // Invert the value because our transistor circuit inverts the PWM signal.
        let speed = u16::MAX - speed;

        let fan: &mut dyn PwmPin<Duty = u16> = match fan_index {
            0 => &mut self.fan_1,
            1 => &mut self.fan_2,
            2 => &mut self.fan_3,
            3 => &mut self.fan_4,
            _ => panic!("Invalid fan index"),
        };

        let adjusted = ((speed as f32 / u16::MAX as f32) * fan.get_max_duty() as f32) as u16;
        fan.set_duty(adjusted);
    }
}
