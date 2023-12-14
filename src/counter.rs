use hal::{
    prelude::*,
    qei::Qei,
    stm32::TIM1,
    timer::{Instant, MonoTimer},
};
use stm32f4xx_hal as hal;

const DETENT_RESET_TIMOUT_MS: u32 = 200;

pub struct Counter<'a, PINS> {
    qei: Qei<TIM1, PINS>,
    // Cache for the count returned by qei for detecting changes
    last_count: u16,
    timer: &'a MonoTimer,
    last_update: Instant,
    diff_accumulator: i16,
}

impl<'a, PINS> Counter<'a, PINS> {
    pub fn new(qei: Qei<TIM1, PINS>, timer: &'a MonoTimer) -> Self {
        let last_count = qei.count();
        Counter { qei, last_count, last_update: timer.now(), timer, diff_accumulator: 0 }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let count = self.qei.count();

        let diff = self.update_counts(count);

        if diff.abs() >= 4 {
            self.diff_accumulator = 0;
            Some((diff / 4) as i8)
        } else {
            None
        }
    }

    // Sometimes the accumulator gets out of sync with the physical detents. We require count to
    // increment by 4 to fire a single tick but when the zero of accumulator lands in between
    // detents (because of noise) we get backlash or missed ticks.
    // Assume that when the dial rests for more than DETENT_RESET_TIMOUT_MS it is aligned to a detent
    // and reset the accumulator.
    fn update_counts(&mut self, count: u16) -> i16 {
        if count != self.last_count {
            self.last_update = self.timer.now();
        }

        if self.accumulator_timed_out() {
            self.diff_accumulator = 0;
        }

        self.diff_accumulator += self.last_count.wrapping_sub(count) as i16;
        self.last_count = count;

        self.diff_accumulator
    }

    fn accumulator_timed_out(&self) -> bool {
        let timeout_ticks =
            self.timer.frequency().0 as f32 * (DETENT_RESET_TIMOUT_MS as f32 / 1000.0);
        self.last_update.elapsed() > timeout_ticks as u32
    }
}
