use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;

// How many up/down edges occur on either the A or B pins from one
// physical detent to the next. Most encoders have 4 edges, but
// ours has 2, and others could have a different number.
const EDGES_PER_DETENT: i8 = 4;

pub struct Counter<A: InputPin, B: InputPin> {
    // qei: Qei<TIM1, PINS>,
    pin_a: A,
    pin_b: B,
    prev_state: (bool, bool),
    pulse_count: i8,
}

impl<A: InputPin<Error=Infallible>, B: InputPin<Error=Infallible>> Counter<A, B> {
    pub fn new(pin_a: A, pin_b: B) -> Self {
        let prev_state = read_state(&pin_a, &pin_b);

        Self {
            pin_a,
            pin_b,
            prev_state,
            pulse_count: 0,
        }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let mut dial_diff = None;

        let curr_state = read_state(&self.pin_a, &self.pin_b);

        // Valid transitions
        // Clockwise:
        //     00 -> 10 -> 11 -> 01 -> 00
        // Counter-clockwise:
        //     00 -> 01 -> 11 -> 10 -> 00
        match (self.prev_state, curr_state) {
            ((false, false), (false, true))
            | ((false, true), (true, true))
            | ((true, true), (true, false))
            | ((true, false), (false, false)) => self.pulse_count -= 1,
            ((false, false), (true, false))
            | ((true, false), (true, true))
            | ((true, true), (false, true))
            | ((false, true), (false, false)) => self.pulse_count += 1,
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

fn read_state<A: InputPin<Error=Infallible>, B: InputPin<Error=Infallible>>(a: &A, b: &B) -> (bool, bool) {
    (a.is_high().unwrap(), b.is_high().unwrap())
}
