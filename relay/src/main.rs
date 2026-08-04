#[cfg(not(target_arch = "wasm32"))]
mod server;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    server::run();
}
