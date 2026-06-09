use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;

const EDGES_PER_DETENT: i8 = 3;

pub struct Counter<A: InputPin, B: InputPin> {
    pin_a: A,
    pin_b: B,
    prev_state: (bool, bool),
    pulse_count: i8,
}

impl<A: InputPin<Error = Infallible>, B: InputPin<Error = Infallible>> Counter<A, B> {
    pub fn new(pin_a: A, pin_b: B) -> Self {
        let prev_state = read_state(&pin_a, &pin_b);

        let pulse_count = match prev_state {
            (true, _) => 0,
            (false, true) => 1,
            (false, false) => -1,
        };

        Self { pin_a, pin_b, prev_state, pulse_count }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let mut dial_diff = None;

        let curr_state = read_state(&self.pin_a, &self.pin_b);

        match (self.prev_state, curr_state) {
            ((true, true), (false, true))
            | ((false, true), (false, false))
            | ((false, false), (true, false)) => self.pulse_count += 1,
            ((true, false), (false, false))
            | ((false, false), (false, true))
            | ((false, true), (true, true)) => self.pulse_count -= 1,
            _ => {},
        }

        if self.pulse_count.abs() >= EDGES_PER_DETENT {
            // Will be -1 or +1
            let diff = self.pulse_count.signum();
            self.pulse_count = 0;
            dial_diff = Some(diff);
        }

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
