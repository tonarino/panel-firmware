use hal::{prelude::*, qei::Qei, stm32::TIM1};
use stm32f4xx_hal as hal;

pub struct Counter<PINS> {
    qei: Qei<TIM1, PINS>,
    last_count: u16,
}

impl<PINS> Counter<PINS> {
    pub fn new(qei: Qei<TIM1, PINS>) -> Self {
        unsafe {
            // TODO(bschwind) - Expose this functionality with a safe interface
            //                  in stm32f4xx-hal.
            // Change the mode of the QEI decoder to mode 1:
            // Counter counts up/down on TI2FP1 edge depending on TI1FP2 level.
            // Or in layman's terms, the encoder counts up and down on encoder
            // pin A edges, while referencing the state of encoder pin B.
            (*TIM1::ptr()).smcr.write(|w| w.sms().encoder_mode_1());
        }

        let last_count = qei.count();
        Counter { qei, last_count }
    }

    pub fn poll(&mut self) -> Option<i8> {
        let count = self.qei.count();
        let diff = count.wrapping_sub(self.last_count) as i16;

        if diff.abs() >= 2 {
            self.last_count = count;
            Some((diff / 2) as i8)
        } else {
            None
        }
    }
}
