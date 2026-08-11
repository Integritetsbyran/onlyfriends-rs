use dioxus::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A shared, async-safe handle to the open account.
pub type AppAccount = Arc<Mutex<client_core::Account>>;

/// Retrieve the account signal from context.
/// Must be called inside a component that is a descendant of a component that has set up the context (should be root.rs).
pub fn use_app_account() -> dioxus::prelude::Signal<AppAccount> {
    use_context::<dioxus::prelude::Signal<AppAccount>>()
}

#[derive(Clone)]
pub struct ModalContent(pub Element);

/// Retrieve the current modal from context.
/// Must be called inside a component that is a descendant of a component that called set up the context (should be root.rs).
pub fn use_modal() -> Signal<Option<ModalContent>> {
    use_context()
}
