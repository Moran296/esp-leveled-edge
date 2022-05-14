use esp_idf_sys::esp_timer_get_time;
use std::time::Duration;

/// A debouncer to resolve interrupt debouncing issues
pub trait Debounce {
    fn is_isr_valid(&mut self) -> bool;
}

/// A debouncer that does nothing
/// Use this if you don't want to debounce
pub struct NoDebounce;
impl Debounce for NoDebounce {
    fn is_isr_valid(&mut self) -> bool {
        true
    }
}

fn micros() -> i64 {
    unsafe { esp_timer_get_time() }
}

/// classic debounce just waits for a certain amount of time to pass
/// between interrupts, so it's not very accurate but works for a lot of cases.
/// most of the time, a 5 to 20 ms debounce time is enough.
pub struct ClassicDebounce {
    debounce_time: i64,
    last_sample: i64,
}

impl ClassicDebounce {
    pub fn new(debounce_time: Duration) -> Self {
        Self {
            debounce_time: debounce_time.as_micros() as i64,
            last_sample: micros(),
        }
    }
}

impl Debounce for ClassicDebounce {
    fn is_isr_valid(&mut self) -> bool {
        let now = micros();
        if now - self.last_sample < self.debounce_time {
            return false;
        }

        self.last_sample = now;
        true
    }
}

/// A debouncer for special cases when the pin can have small glitches when it is not bouncing
/// It works well for rotary encoders with about 20 ms debounce time
pub struct FilterDebounce {
    debounce_time: i64,
    last_sample: i64,
    ignore_next: bool,
}

impl FilterDebounce {
    pub fn new(debounce_time: Duration) -> Self {
        Self {
            debounce_time: debounce_time.as_micros() as i64,
            last_sample: micros(),
            ignore_next: false,
        }
    }
}

impl Debounce for FilterDebounce {
    #[inline(always)]
    #[link_section = ".iram1.filter_bouncer"]
    fn is_isr_valid(&mut self) -> bool {
        let mut is_passed = false;
        let now = micros();

        if !self.ignore_next {
            if now - self.last_sample < self.debounce_time {
                self.ignore_next = true;
            } else {
                is_passed = true;
            }
        } else {
            self.ignore_next = false;
        }

        self.last_sample = now;

        is_passed
    }
}
