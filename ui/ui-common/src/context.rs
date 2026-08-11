use std::sync::Arc;

use dioxus::prelude::*;
use tokio::sync::Mutex;

/// A shared, async-safe handle to the open account.
///
/// `PartialEq` is by pointer identity (two handles are equal iff they point to
/// the same allocation), which is the right semantic for Dioxus memoization: the
/// account is created once on login and the Arc never changes.
#[derive(Clone)]
pub struct AppAccount(Arc<Mutex<client_core::Account>>);

impl AppAccount {
    pub fn new(account: client_core::Account) -> Self {
        Self(Arc::new(Mutex::new(account)))
    }
}

impl PartialEq for AppAccount {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for AppAccount {
    type Target = Arc<Mutex<client_core::Account>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Retrieve the account signal from context.
/// Only callable inside a descendant of AppRoot (which provides this context).
pub fn use_app_account() -> dioxus::prelude::Signal<AppAccount> {
    use_context::<dioxus::prelude::Signal<AppAccount>>()
}

#[derive(Clone)]
pub struct ModalContent(pub Element);

/// Retrieve the current modal from context.
pub fn use_modal() -> Signal<Option<ModalContent>> {
    use_context()
}
