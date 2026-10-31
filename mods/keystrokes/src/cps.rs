//! Clicks per second: presses within the last second, on the frame clock the host provides.

/// The window CPS is counted over.
const WINDOW: f64 = 1.0;
/// More presses than this in a second are not a human clicking; the buffer stays bounded.
const MAX_PRESSES: usize = 64;

#[derive(Debug, Default)]
pub struct Counter {
    /// Times of recent presses, oldest first.
    presses: Vec<f64>,
}

impl Counter {
    /// Records `count` presses at `now`, forgets those older than the window, and returns
    /// how many remain.
    pub fn tick(&mut self, now: f64, count: usize) -> u32 {
        for _ in 0..count {
            if self.presses.len() >= MAX_PRESSES {
                self.presses.remove(0);
            }
            self.presses.push(now);
        }
        let keep = self.presses.iter().position(|time| now - time < WINDOW).unwrap_or(self.presses.len());
        self.presses.drain(..keep);
        self.presses.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_count_for_one_second_then_expire() {
        let mut counter = Counter::default();
        assert_eq!(counter.tick(0.0, 1), 1);
        assert_eq!(counter.tick(0.3, 2), 3);
        assert_eq!(counter.tick(0.9, 0), 3);
        assert_eq!(counter.tick(1.05, 0), 2, "the first press has aged out");
        assert_eq!(counter.tick(2.0, 0), 0);
    }

    #[test]
    fn the_buffer_never_grows_without_bound() {
        let mut counter = Counter::default();
        assert_eq!(counter.tick(0.0, 500), MAX_PRESSES as u32);
    }
}
