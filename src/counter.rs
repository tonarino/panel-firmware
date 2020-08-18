use hal::{prelude::*, qei::Qei, stm32::TIM2, timer::Tim2NoRemap};
use stm32f1xx_hal as hal;

pub struct Counter<PINS> {
    qei: Qei<TIM2, Tim2NoRemap, PINS>,
    last_count: u16,
}

impl<PINS> Counter<PINS> {
    pub fn new(qei: Qei<TIM2, Tim2NoRemap, PINS>) -> Self {
        let last_count = qei.count();
        Counter { qei, last_count }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let count = self.qei.count();
        let diff = count.wrapping_sub(self.last_count) as i16;

        if diff.abs() >= 4 {
            self.last_count = count;
            Some((diff / 4) as i8)
        } else {
            None
        }
    }
}
