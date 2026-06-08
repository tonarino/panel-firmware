use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;

pub struct Counter<A: InputPin, B: InputPin> {
    pin_a: A,
    pin_b: B,
    prev_state: (bool, bool),
}

impl<A: InputPin<Error = Infallible>, B: InputPin<Error = Infallible>> Counter<A, B> {
    pub fn new(pin_a: A, pin_b: B) -> Self {
        let prev_state = read_state(&pin_a, &pin_b);

        Self { pin_a, pin_b, prev_state }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let curr_state = read_state(&self.pin_a, &self.pin_b);

        // We only count a pulse on the rising or falling edge of B, and
        // the direction depends on the level of A, which should be unchanged
        // as B rises or falls. The falling edge of B should be roughly in the
        // middle of the two detents, according to the Alps EC20A datasheet.
        let dial_diff = match (self.prev_state, curr_state) {
            ((false, true), (false, false)) => Some(1),
            ((false, false), (false, true)) => Some(-1),
            _ => None,
        };

        self.prev_state = curr_state;

        dial_diff
    }
}

fn read_state(
    a: &dyn InputPin<Error = Infallible>,
    b: &dyn InputPin<Error = Infallible>,
) -> (bool, bool) {
    (a.is_high().unwrap(), b.is_high().unwrap())
}
