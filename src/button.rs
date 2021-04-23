use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;

pub struct Button<T: InputPin> {
    pin: Debouncer<T>,
    button_state: ButtonState,
}

pub enum ButtonEvent {
    /// The button has just been pressed down.
    Press,

    /// The button was released.
    Release,
}

enum ButtonState {
    Released,
    Pressed,
}

impl<T: InputPin<Error = Infallible>> Button<T> {
    pub fn new(pin: Debouncer<T>) -> Self {
        let button_state = ButtonState::Released;

        Self { pin, button_state }
    }

    pub fn is_pressed(&self) -> bool {
        self.pin.is_pressed()
    }

    pub fn poll(&mut self) -> Option<ButtonEvent> {
        self.pin.poll();

        match self.button_state {
            ButtonState::Released => {
                if self.pin.is_pressed() {
                    self.button_state = ButtonState::Pressed;
                    return Some(ButtonEvent::Press);
                }
            },
            ButtonState::Pressed => {
                if !self.pin.is_pressed() {
                    self.button_state = ButtonState::Released;
                    return Some(ButtonEvent::Release);
                }
            },
        }

        None
    }
}

// Debouncer code inspired by Kenneth Kuhn's C debouncer:
// http://www.kennethkuhn.com/electronics/debounce.c
pub struct Debouncer<T: InputPin> {
    pin: T,
    integrator: u8,
    max: u8,
    output: bool,
    active_mode: Active,
}

#[allow(dead_code)]
pub enum Active {
    Low,
    High,
}

impl<T: InputPin<Error = Infallible>> Debouncer<T> {
    pub fn new(pin: T, active_mode: Active, debounce_time_ms: u16, sample_frequency: u16) -> Self {
        let max = ((debounce_time_ms as f32 / 1000.0) * sample_frequency as f32) as u8;

        let integrator = match active_mode {
            Active::Low => max,
            Active::High => 0,
        };

        let output = match active_mode {
            Active::Low => true,
            Active::High => false,
        };

        Self { pin, integrator, max, output, active_mode }
    }

    pub fn poll(&mut self) {
        if self.pin.is_low().unwrap() {
            self.integrator = self.integrator.saturating_sub(1);
        } else if self.integrator < self.max {
            self.integrator += 1;
        }

        if self.integrator == 0 {
            self.output = false;
        } else if self.integrator >= self.max {
            self.output = true;
        }
    }

    pub fn is_pressed(&self) -> bool {
        matches!((&self.active_mode, self.output), (Active::High, true) | (Active::Low, false))
    }
}
