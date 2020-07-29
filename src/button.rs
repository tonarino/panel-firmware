use crate::debouncer::Debouncer;
use core::convert::Infallible;
use stm32f1xx_hal as hal;

use cortex_m::peripheral::DWT;
use embedded_hal::digital::v2::InputPin;
use hal::{
    rcc::Clocks,
    time::{Instant, MonoTimer},
};

pub struct LongPressButton<T: InputPin> {
    pin: Debouncer<T>,
    timer: MonoTimer,
    button_state: ButtonState,
    long_press_timeout_ticks: u32,
}

pub enum ButtonEvent {
    /// The button has just been pressed down.
    Pressed,

    /// The button was released before the "long press" timeout.
    ShortRelease,

    /// The button has been held for at least the "long press" timeout.
    LongPress,

    /// The button has been released after a "long press".
    LongRelease,
}

enum ButtonState {
    Released,
    Pressed(Instant),
    LongPressed,
}

impl<T: InputPin<Error = Infallible>> LongPressButton<T> {
    pub fn new(pin: Debouncer<T>, long_press_timeout_ms: u32, dwt: DWT, clocks: Clocks) -> Self {
        let timer = MonoTimer::new(dwt, clocks);
        let button_state = ButtonState::Released;
        let long_press_timeout_ticks =
            (timer.frequency().0 as f32 * (long_press_timeout_ms as f32 / 1000.0)) as u32;

        Self { pin, timer, button_state, long_press_timeout_ticks }
    }

    pub fn update(&mut self) -> Option<ButtonEvent> {
        self.pin.update();

        match self.button_state {
            ButtonState::Released => {
                if self.pin.is_pressed() {
                    let now = self.timer.now();
                    self.button_state = ButtonState::Pressed(now);
                    return Some(ButtonEvent::Pressed);
                }
            },
            ButtonState::Pressed(press_start) => {
                // if press_start.elapsed() > self.long_press_timeout_ticks {
                //     self.button_state = ButtonState::LongPressed;
                //     return Some(ButtonEvent::LongPress);
                // }

                // if !self.pin.is_pressed() {
                //     self.button_state = ButtonState::Released;
                //     return Some(ButtonEvent::ShortRelease);
                // }

                if !self.pin.is_pressed() {
                    self.button_state = ButtonState::Released;
                    return Some(ButtonEvent::ShortRelease);
                } else if press_start.elapsed() > self.long_press_timeout_ticks {
                    self.button_state = ButtonState::LongPressed;
                    return Some(ButtonEvent::LongPress);
                }
            },
            ButtonState::LongPressed => {
                if !self.pin.is_pressed() {
                    self.button_state = ButtonState::Released;
                    return Some(ButtonEvent::LongRelease);
                }
            },
        }

        None
    }
}
