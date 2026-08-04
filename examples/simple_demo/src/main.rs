#[cfg(not(target_arch = "wasm32"))]
mod app;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    app::run().unwrap();
}
