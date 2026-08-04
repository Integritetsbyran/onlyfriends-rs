#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod views;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    app::run();
}
