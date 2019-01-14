use std::{thread, time};

#[derive(Default)]
pub struct Backoff {
    count: u8,
}

const ONE: time::Duration = time::Duration::from_millis(1);
const FOUR: time::Duration = time::Duration::from_millis(4);
const EIGHT: time::Duration = time::Duration::from_millis(8);
const SIXTEEN: time::Duration = time::Duration::from_millis(16);
// const THIRTYTWO: time::Duration = time::Duration::from_millis(32);
// const SIXTYFOUR: time::Duration = time::Duration::from_millis(64);
// const ONETWOEIGHT: time::Duration = time::Duration::from_millis(128);
// const TWOFIVESIX: time::Duration = time::Duration::from_millis(256);
// const FIVEONETWO: time::Duration = time::Duration::from_millis(512);

impl Backoff {
    pub fn reset(&mut self) -> () {
        self.count = 0;
    }

    pub fn delay(&mut self) -> () {
        match self.count {
            0 => (),
            1 => thread::sleep(ONE),
            2 => thread::sleep(FOUR),
            3 => thread::sleep(EIGHT),
            _ => thread::sleep(SIXTEEN),
            // 5 => thread::sleep(THIRTYTWO),
            // 6 => thread::sleep(SIXTYFOUR),
            // 7 => thread::sleep(ONETWOEIGHT),
            // 8 => thread::sleep(TWOFIVESIX),
            // _ => thread::sleep(FIVEONETWO),
        }
        self.count = self.count.saturating_add(1);
    }
}
