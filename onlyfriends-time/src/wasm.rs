use web_sys::js_sys;

pub struct WasmTime;

impl WasmTime {
    pub fn epoch_secs() -> u64 {
        (js_sys::Date::now() / 1000.0).floor() as u64
    }
}
