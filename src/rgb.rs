use core::ops::{Add, Mul};

#[derive(Copy, Clone)]
pub struct Rgb {
    /// All in the range 0.0 - 255.0
    /// Rounded when actually in use
    r: f32,
    g: f32,
    b: f32,
}

impl Rgb {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn new_from_u8(r: u8, g: u8, b: u8) -> Self {
        Self::new(r as f32, g as f32, b as f32)
    }

    /// Fade the current color toward the other one with a simple moving average
    pub fn fade_towards(&mut self, other: &Self, fade_const: f32) {
        *self = *self * fade_const + *other * (1.0 - fade_const);
    }

    pub fn r(&self) -> u8 {
        self.r.clamp(0.0, 255.0) as u8
    }

    pub fn g(&self) -> u8 {
        self.g.clamp(0.0, 255.0) as u8
    }

    pub fn b(&self) -> u8 {
        self.b.clamp(0.0, 255.0) as u8
    }
}

impl Mul<f32> for Rgb {
    type Output = Rgb;

    fn mul(self, rhs: f32) -> Self::Output {
        Rgb::new(self.r * rhs, self.g * rhs, self.b * rhs)
    }
}

impl Add<Rgb> for Rgb {
    type Output = Rgb;

    fn add(self, rhs: Rgb) -> Self::Output {
        Rgb::new(self.r + rhs.r, self.g + rhs.g, self.b + rhs.b)
    }
}
