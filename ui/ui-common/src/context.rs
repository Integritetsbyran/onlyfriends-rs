use dioxus::prelude::*;
use std::sync::Arc;
use storage_common::storage::Storage;
use tokio::sync::Mutex;

/// A shared, async-safe handle to the open account.
pub type AppAccount<Store: Storage> = Arc<Mutex<client_core::Account<Store>>>;

/// Retrieve the account signal from context. Must be called inside a component
/// that is a descendant of a component that called
/// `use_context_provider(|| Signal::new(None::<AppAccount<Store>>))`.
pub fn use_app_account<Store: Storage + 'static>(
) -> dioxus::prelude::Signal<Option<AppAccount<Store>>> {
    use_context::<dioxus::prelude::Signal<Option<AppAccount<Store>>>>()
}
