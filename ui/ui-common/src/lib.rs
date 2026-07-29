//! Shared UI components and pages for all platforms.

pub mod components;
pub mod context;
pub mod hex_util;
pub mod pages;

mod assets;
pub use assets::APP_CSS;

pub mod hero;
pub use hero::Hero;
pub mod navbar;
pub use navbar::Navbar;
