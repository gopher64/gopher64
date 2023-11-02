#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[cfg(web_sys_unstable_apis)]
#[wasm_bindgen]
extern "C" {
    # [wasm_bindgen (extends = EventTarget , extends = :: js_sys :: Object , js_name = XRSession , typescript_type = "XRSession")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSession` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub type XrSession;
    #[cfg(web_sys_unstable_apis)]
    #[cfg(feature = "XrVisibilityState")]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = visibilityState)]
    #[doc = "Getter for the `visibilityState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/visibilityState)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`, `XrVisibilityState`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn visibility_state(this: &XrSession) -> XrVisibilityState;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = frameRate)]
    #[doc = "Getter for the `frameRate` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/frameRate)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn frame_rate(this: &XrSession) -> Option<f32>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = supportedFrameRates)]
    #[doc = "Getter for the `supportedFrameRates` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/supportedFrameRates)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn supported_frame_rates(this: &XrSession) -> Option<Vec<f32>>;
    #[cfg(web_sys_unstable_apis)]
    #[cfg(feature = "XrRenderState")]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = renderState)]
    #[doc = "Getter for the `renderState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/renderState)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrRenderState`, `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn render_state(this: &XrSession) -> XrRenderState;
    #[cfg(web_sys_unstable_apis)]
    #[cfg(feature = "XrInputSourceArray")]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = inputSources)]
    #[doc = "Getter for the `inputSources` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/inputSources)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrInputSourceArray`, `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn input_sources(this: &XrSession) -> XrInputSourceArray;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onend)]
    #[doc = "Getter for the `onend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onend(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onend)]
    #[doc = "Setter for the `onend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onend(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = oninputsourceschange)]
    #[doc = "Getter for the `oninputsourceschange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/oninputsourceschange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn oninputsourceschange(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = oninputsourceschange)]
    #[doc = "Setter for the `oninputsourceschange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/oninputsourceschange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_oninputsourceschange(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onselect)]
    #[doc = "Getter for the `onselect` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselect)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onselect(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onselect)]
    #[doc = "Setter for the `onselect` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselect)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onselect(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onselectstart)]
    #[doc = "Getter for the `onselectstart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectstart)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onselectstart(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onselectstart)]
    #[doc = "Setter for the `onselectstart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectstart)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onselectstart(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onselectend)]
    #[doc = "Getter for the `onselectend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onselectend(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onselectend)]
    #[doc = "Setter for the `onselectend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onselectend(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onsqueeze)]
    #[doc = "Getter for the `onsqueeze` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueeze)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onsqueeze(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onsqueeze)]
    #[doc = "Setter for the `onsqueeze` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueeze)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onsqueeze(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onsqueezestart)]
    #[doc = "Getter for the `onsqueezestart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezestart)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onsqueezestart(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onsqueezestart)]
    #[doc = "Setter for the `onsqueezestart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezestart)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onsqueezestart(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onsqueezeend)]
    #[doc = "Getter for the `onsqueezeend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezeend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onsqueezeend(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onsqueezeend)]
    #[doc = "Setter for the `onsqueezeend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezeend)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onsqueezeend(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onvisibilitychange)]
    #[doc = "Getter for the `onvisibilitychange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onvisibilitychange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onvisibilitychange(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onvisibilitychange)]
    #[doc = "Setter for the `onvisibilitychange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onvisibilitychange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onvisibilitychange(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , getter , js_class = "XRSession" , js_name = onframeratechange)]
    #[doc = "Getter for the `onframeratechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onframeratechange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn onframeratechange(this: &XrSession) -> Option<::js_sys::Function>;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (structural , method , setter , js_class = "XRSession" , js_name = onframeratechange)]
    #[doc = "Setter for the `onframeratechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onframeratechange)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn set_onframeratechange(this: &XrSession, value: Option<&::js_sys::Function>);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = cancelAnimationFrame)]
    #[doc = "The `cancelAnimationFrame()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/cancelAnimationFrame)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn cancel_animation_frame(this: &XrSession, handle: u32);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = end)]
    #[doc = "The `end()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/end)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn end(this: &XrSession) -> ::js_sys::Promise;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = requestAnimationFrame)]
    #[doc = "The `requestAnimationFrame()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestAnimationFrame)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn request_animation_frame(this: &XrSession, callback: &::js_sys::Function) -> u32;
    #[cfg(web_sys_unstable_apis)]
    #[cfg(feature = "XrReferenceSpaceType")]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = requestReferenceSpace)]
    #[doc = "The `requestReferenceSpace()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestReferenceSpace)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrReferenceSpaceType`, `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn request_reference_space(
        this: &XrSession,
        type_: XrReferenceSpaceType,
    ) -> ::js_sys::Promise;
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = updateRenderState)]
    #[doc = "The `updateRenderState()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateRenderState)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn update_render_state(this: &XrSession);
    #[cfg(web_sys_unstable_apis)]
    #[cfg(feature = "XrRenderStateInit")]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = updateRenderState)]
    #[doc = "The `updateRenderState()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateRenderState)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrRenderStateInit`, `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn update_render_state_with_state(this: &XrSession, state: &XrRenderStateInit);
    #[cfg(web_sys_unstable_apis)]
    # [wasm_bindgen (method , structural , js_class = "XRSession" , js_name = updateTargetFrameRate)]
    #[doc = "The `updateTargetFrameRate()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateTargetFrameRate)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `XrSession`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn update_target_frame_rate(this: &XrSession, rate: f32) -> ::js_sys::Promise;
}
