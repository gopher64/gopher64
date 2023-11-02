use mach2::mach_time::{mach_absolute_time, mach_timebase_info};

#[derive(Clone, Copy, Debug)]
pub struct Monotonic {
    factor: u64,
}
impl Monotonic {
    pub fn now(&self) -> u64 {
        let raw = unsafe { mach_absolute_time() };
        raw * self.factor
    }
}

impl Default for Monotonic {
    fn default() -> Self {
        let mut info = mach_timebase_info { numer: 0, denom: 0 };
        unsafe {
            mach_timebase_info(&mut info);
        }

        let factor = u64::from(info.numer) / u64::from(info.denom);
        Self { factor }
    }
}
