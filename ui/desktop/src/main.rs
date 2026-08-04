#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod config;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    app::run();
}
