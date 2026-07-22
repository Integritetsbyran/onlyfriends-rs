#[cfg(not(target_arch = "wasm32"))]
mod normal;
#[cfg(not(target_arch = "wasm32"))]
use normal::NormalTime as Time;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::WasmTime as Time;

const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

/// Returns the number of seconds since the Unix epoch (January 1, 1970) UTC.
#[inline(always)]
pub fn seconds_since_epoch() -> u64 {
    Time::epoch_secs()
}

/// Returns the number of days since the Unix epoch (January 1, 1970) UTC.
#[inline(always)]
pub fn days_since_epoch() -> u64 {
    seconds_since_epoch() / SECONDS_PER_DAY
}
