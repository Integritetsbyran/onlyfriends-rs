use std::sync::Arc;
use tokio::sync::Mutex;

/// A shared, async-safe handle to the open account.
pub type AppAccount = Arc<Mutex<client_core::Account>>;

/// Retrieve the account signal from context. Must be called inside a component
/// that is a descendant of a component that called
/// `use_context_provider(|| Signal::new(None::<AppAccount>))`.
pub fn use_app_account() -> dioxus::prelude::Signal<Option<AppAccount>> {
    dioxus::prelude::use_context::<dioxus::prelude::Signal<Option<AppAccount>>>()
}
