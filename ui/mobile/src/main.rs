use client_core::account::Store;
use dioxus::prelude::*;
use dioxus_router::hooks::use_navigator;
use dioxus_router::{Link, Outlet, Routable, Router};
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
    #[cfg(feature = "native")]
    {
        use dioxus_native::prelude::*;
        // Use Blitz native renderer (experimental)
        // NOTE: Does not support iOS safe areas yet
        dioxus_native::launch(App);
    }
    
    #[cfg(not(feature = "native"))]
    {
        // Use WebView renderer (default)
        // Supports CSS env() for safe areas on iOS/Android
        dioxus::launch(App);
    }
}

/// Android native entrypoint used by `android-activity`.
#[cfg(target_os = "android")]
#[cfg(not(feature = "native"))]
#[unsafe(no_mangle)]
pub fn android_main(android_app: dioxus::mobile::wry::AndroidApp) {
    use dioxus::mobile::wry::prelude::*;
    use dioxus::mobile::Config;
    
    const BACKGROUND_COLOR: (u8, u8, u8, u8) = (18, 18, 18, 255);
    
    let config = Config::new()
        .with_background_color(BACKGROUND_COLOR)
        .with_on_window(|_window, _| {
            dispatch(|env, activity, _webview| {
                let window = env
                    .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
                    .unwrap()
                    .l()
                    .unwrap();

                let decor_view = env
                    .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
                    .unwrap()
                    .l()
                    .unwrap();

                // Draw UNDER the status bar (edge-to-edge)
                const LAYOUT_STABLE: i32 = 1 << 8;
                const LAYOUT_HIDE_NAVIGATION: i32 = 1 << 9;
                const LAYOUT_FULLSCREEN: i32 = 1 << 10;
                const VISIBILITY_FLAG: i32 =
                    LAYOUT_STABLE | LAYOUT_FULLSCREEN | LAYOUT_HIDE_NAVIGATION;

                env.call_method(
                    decor_view,
                    "setSystemUiVisibility",
                    "(I)V",
                    &[VISIBILITY_FLAG.into()],
                )
                .unwrap();

                // Make the status bars transparent
                const TRANSPARENT_COLOR: i32 = 0;
                env.call_method(
                    &window,
                    "setStatusBarColor",
                    "(I)V",
                    &[TRANSPARENT_COLOR.into()],
                )
                .unwrap();

                env.call_method(
                    &window,
                    "setNavigationBarColor",
                    "(I)V",
                    &[TRANSPARENT_COLOR.into()],
                )
                .unwrap();
            });
        });

    dioxus::mobile::launch_cfg(App, config);
}

/// Root component — provides the account context for the entire tree.
#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(None::<context::AppAccount>));
    use_context_provider(|| Signal::new(None::<context::ModalContent>));

    rsx! {
        // Enable edge-to-edge layout with safe area support for iOS
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1.0, viewport-fit=cover"
        }
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
