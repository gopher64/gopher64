//! Bindings to the JS API.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

#[wasm_bindgen]
extern "C" {
	type Global;

	#[wasm_bindgen(method, getter)]
	fn performance(this: &Global) -> JsValue;

	pub(super) type Performance;

	#[wasm_bindgen(method)]
	pub(super) fn now(this: &Performance) -> f64;

	#[cfg(target_feature = "atomics")]
	#[wasm_bindgen(method, getter, js_name = timeOrigin)]
	pub(super) fn time_origin(this: &Performance) -> f64;
}

thread_local! {
	pub(super) static PERFORMANCE: Performance = {
		let global: Global = js_sys::global().unchecked_into();
		let performance = global.performance();

		if performance.is_undefined() {
			panic!("`Performance` object not found")
		} else {
			performance.unchecked_into()
		}
	};
}
