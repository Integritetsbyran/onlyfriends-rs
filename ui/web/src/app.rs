use dioxus::prelude::*;

use client_core::account::Store;
use std::sync::Arc;
use storage_web::WebStorage;
use tokio::sync::Mutex;
use ui::{APP_CSS, context, pages};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    /// First-run onboarding.
    #[route("/setup")]
    Setup {},
    #[layout(AppLayout)]
        /// Startup guard — tries auto-login, then redirects.
        #[route("/")]
        Guard {},
        /// Main feed.
        #[route("/feed")]
        Feed {},
        /// Friends management.
        #[route("/friends")]
        Friends {},
        /// Own profile.
        #[route("/profile")]
        Profile {},
        /// App preferences.
        #[route("/prefs")]
        Prefs {},
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: APP_CSS }
        Router::<Route> {}
    }
}

/// Shared layout — owns the account lifecycle.
#[component]
fn AppLayout() -> Element {
    let mut account = use_signal(|| None::<context::AppAccount>);
    let mut initialized = use_signal(|| false);

    // Make the writable optional signal available as context so the Setup route
    // can set it once registration or auto-login completes.
    use_context_provider(|| account);

    let nav = use_navigator();

    use_effect(move || {
        spawn(async move {
            let store: Store = Arc::new(Mutex::new(WebStorage::open("TMP").await.unwrap()));
            match client_core::Account::open(store).await {
                Ok(Some(acc)) => {
                    account.set(Some(context::AppAccount::new(acc)));
                    initialized.set(true);
                }
                _ => {
                    nav.push(Route::Setup {});
                }
            }
        });
    });

    if !initialized() {
        return rsx! { div { class: "loading", "Loading…" } };
    }

    match account() {
        Some(acc) => rsx! {
            ui::AppRoot {
                on_feed: move |_| { nav.push(Route::Feed {}); },
                on_friends: move |_| { nav.push(Route::Friends {}); },
                on_profile: move |_| { nav.push(Route::Profile {}); },
                on_prefs: move |_| { nav.push(Route::Prefs {}); },
                account: acc,
                Outlet::<Route> {}
            }
        },
        None => rsx! { div { class: "loading", "Loading…" } },
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
    let mut storage = use_signal(|| None::<Store>);

    use_effect(move || {
        spawn(async move {
            let s: Store = Arc::new(Mutex::new(WebStorage::open("TMP").await.unwrap()));
            storage.set(Some(s));
        });
    });

    rsx! {
        if let Some(store) = storage() {
            pages::SetupPage {
                on_complete: move |()| {
                    nav.push(Route::Feed {});
                },
                get_storage: move || store.clone(),
            }
        } else {
            div { class: "loading", "Loading…" }
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
    let copy = use_callback(async move |key: String| {
        let js = format!(
            r#"
const text = "{key}";
await navigator.clipboard.writeText(text);
dioxus.send(true);                
            "#
        );

        let mut eval = document::eval(&js);
        if eval.recv::<bool>().await.is_ok() {}
    });

    rsx! {
        pages::FriendsPage {
            on_copy_key: move |key| {
                spawn(async move {
                    copy.call(key).await;
                });
            },
        }
    }
}

#[component]
fn Profile() -> Element {
    rsx! {
        pages::ProfilePage {}
    }
}

#[component]
fn Prefs() -> Element {
    rsx! {
        pages::Prefs {}
    }
}
