use client_core::account::Store;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

use storage_sqlite::SqliteStorage;
use ui::{context, pages};

mod config;

const APP_CSS: Asset = ui::APP_CSS;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppLayout)]
        /// Startup guard — tries auto-login, then redirects.
        #[route("/")]
        Guard {},
        /// First-run onboarding.
        #[route("/setup")]
        Setup {},
        /// Main feed.
        #[route("/feed")]
        Feed {},
        /// Friends management.
        #[route("/friends")]
        Friends {},
        /// Own profile.
        #[route("/profile")]
        Profile {},
        /// Confirm-add screen reached via an "add friend" deep link (a
        /// scanned QR code / universal link / onlyfriends:// custom scheme).
        #[route("/add/:code")]
        AddFriend { code: String },
}

fn main() {
    dioxus::launch(App);
}

/// Root component — provides the account context for the entire tree.
#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(None::<context::AppAccount>));
    use_context_provider(|| Signal::new(None::<context::ModalContent>));

    rsx! {
        document::Stylesheet { href: APP_CSS }
        Router::<Route> {}
    }
}

/// Shared layout. Shows the top-nav only when the user is logged in.
#[component]
fn AppLayout() -> Element {
    let account = context::use_app_account();

    rsx! {
        div { class: "app-root",
            if account.read().is_some() {
                nav { class: "top-nav",
                    span { class: "app-title", "OnlyFriends" }
                    Link { to: Route::Feed {}, class: "nav-tab", "Feed" }
                    Link { to: Route::Friends {}, class: "nav-tab", "Friends" }
                    Link { to: Route::Profile {}, class: "nav-tab", "Profile" }
                }
            }
            Outlet::<Route> {}
        }
    }
}

/// Startup redirect — always sends new sessions to Setup.
/// The relay URL lives only in memory, so there is nothing to restore.
#[component]
fn Guard() -> Element {
    let nav = use_navigator();
    let mut did_init = use_signal(|| false);

    use_effect(move || {
        if did_init() {
            return;
        }
        did_init.set(true);
        nav.push(Route::Setup {});
    });

    rsx! {
        div { class: "loading", "Loading…" }
    }
}

/// Onboarding wrapper — delegates to shared SetupPage, then navigates.
#[component]
fn Setup() -> Element {
    let nav = use_navigator();
    let storage: Store = Arc::new(Mutex::new(SqliteStorage::open(&config::db_path()).unwrap()));
    let callback = use_callback(move |_| storage.clone());
    // Cold-start only (see `find_add_friend_arg` docs); does not cover
    // Android `onNewIntent`/iOS `openURL` while already running.
    let pending_deep_link = client_core::deep_link::find_add_friend_arg(std::env::args());
    rsx! {
        pages::SetupPage {
            on_complete: move |_| {
                match pending_deep_link.clone() {
                    Some(code) => nav.push(Route::AddFriend { code }),
                    None => nav.push(Route::Feed {}),
                };
            },
            get_storage: callback,
        }
    }
}

#[component]
fn Feed() -> Element {
    rsx! {
        pages::FeedPage {}
    }
}

#[component]
fn Friends() -> Element {
    rsx! {
        pages::FriendsPage {}
    }
}

#[component]
fn Profile() -> Element {
    rsx! {
        pages::ProfilePage {}
    }
}

/// Confirm-add screen reached via an "add friend" deep link.
#[component]
fn AddFriend(code: String) -> Element {
    let nav = use_navigator();
    rsx! {
        pages::AddFriendPage {
            code,
            on_added: move |_| {
                nav.push(Route::Friends {});
            },
        }
    }
}
