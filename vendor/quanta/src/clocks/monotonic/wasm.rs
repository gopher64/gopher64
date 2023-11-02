const WASM_WRONG_ENV: &str = "failed to find the global `window` object: the `wasm32-unknown-unknown` implementation only supports running in web browsers; wse `wasm32-wasi` to run elsewhere";
const WASM_MISSING_WINDOW_PERF: &str = "failed to find `window.performance`";

#[derive(Clone, Copy, Debug, Default)]
pub struct Monotonic;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl Monotonic {
    pub fn now(&self) -> u64 {
        let now = web_sys::window()
            .expect(WASM_WRONG_ENV)
            .performance()
            .expect(WASM_MISSING_WINDOW_PERF)
            .now();
        // `window.performance.now()` returns the time in milliseconds.
        return f64::trunc(now * 1_000_000.0) as u64;
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
impl Monotonic {
    pub fn now(&self) -> u64 {
        unsafe { wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 1).expect("failed to get time") }
    }
}
