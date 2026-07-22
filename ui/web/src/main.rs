#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
pub mod views;

fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus::launch(app::App);
}
