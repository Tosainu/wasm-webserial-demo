use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(a: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::util::log(&format_args!($($t)*).to_string()))
}

pub(crate) use console_log;

// https://developer.chrome.com/docs/capabilities/serial#feature-detection
#[wasm_bindgen(inline_js = r#"
    export function web_serial_api_supported() {
        return 'serial' in navigator;
    }"#)]
extern "C" {
    pub fn web_serial_api_supported() -> bool;
}

// https://developer.chrome.com/docs/capabilities/serial#feature-detection
#[wasm_bindgen(inline_js = r#"
    export function file_system_access_api_supported() {
        return 'showOpenFilePicker' in self;
    }"#)]
extern "C" {
    pub fn file_system_access_api_supported() -> bool;
}
