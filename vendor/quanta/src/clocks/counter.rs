#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
use core::arch::x86_64::_rdtsc;

#[derive(Clone, Debug, Default)]
pub struct Counter;

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
impl Counter {
    pub fn now(&self) -> u64 {
        unsafe { _rdtsc() }
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
impl Counter {
    pub fn now(&self) -> u64 {
        panic!("can't use counter without TSC support");
    }
}
