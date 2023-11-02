/// Handles dynamic thread sleep time.
pub struct SleepAmountTracker {
    /// The maximum amount of time for thread to sleep.
    max_time_to_sleep: u8,
    /// Current time to block the thread.
    time_to_sleep: u8,
    /// The current warm wakeup number.
    warm_wakeup: u8,
    /// The maximum amount of clipboard wakeups in a row with low sleep amount.
    max_warm_wakeups: u8,
}

impl SleepAmountTracker {
    /// Build new tracker for sleep amount.
    ///
    /// `max_time_to_sleep` - maximum sleep value for a thread.
    /// ``
    pub fn new(max_time_to_sleep: u8, max_warm_wakeups: u8) -> Self {
        Self { max_time_to_sleep, max_warm_wakeups, warm_wakeup: 0, time_to_sleep: 0 }
    }

    /// Reset the current sleep amount to 0ms.
    #[inline]
    pub fn reset_sleep(&mut self) {
        self.time_to_sleep = 0;
    }

    /// Adjust the sleep amount.
    #[inline]
    pub fn increase_sleep(&mut self) {
        if self.time_to_sleep == 0 {
            // Reset `time_to_sleep` to one, so we can reach `max_time_to_sleep`.
            self.time_to_sleep = 1;
            // Reset `warm_wakeup` count.
            self.warm_wakeup = 0;

            return;
        }

        if self.warm_wakeup < self.max_warm_wakeups {
            // Handled warm wake up.
            self.warm_wakeup += 1;
        } else if self.time_to_sleep < self.max_warm_wakeups {
            // The aim of this different sleep times is to provide a good performance under
            // high the load and not waste system resources too much when idle.
            self.time_to_sleep = std::cmp::min(2 * self.time_to_sleep, self.max_time_to_sleep);
        }
    }

    /// Get the current time to sleep in ms.
    #[inline]
    pub fn sleep_amount(&self) -> u8 {
        self.time_to_sleep
    }
}
