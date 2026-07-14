use dioxus::prelude::*;

/// Shared dark-mode stylesheet. Use this constant in platform crates so
/// the `asset!` path resolves relative to the ui-common crate directory.
pub const APP_CSS: Asset = asset!("/assets/styling/app.css");
