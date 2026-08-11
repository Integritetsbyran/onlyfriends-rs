use client_core::account::Store;
use dioxus_native::prelude::*;
use dioxus_router::hooks::use_navigator;
use dioxus_router::{Outlet, Routable, Router};
use image::GenericImageView;
use std::sync::Arc;
use storage_sqlite::SqliteStorage;
use tokio::sync::Mutex;
use ui::{context, pages};

use crate::config;

const APP_CSS: Asset = ui::APP_CSS;

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

pub fn run() {
    const ICON: &[u8] = include_bytes!("../../ui-common/assets/icon_x128.png");
    let icon = image::load_from_memory_with_format(ICON, image::ImageFormat::Png).unwrap();
    let (width, height) = icon.dimensions();
    let icon = winit::icon::RgbaIcon::new(icon.into_rgba8().into_vec(), width, height).unwrap();

    let wayland_attrs = winit_wayland::WindowAttributesWayland::default()
        // Set application id. This decides the icon on Linux/Wayland.
        .with_name("org.integritetsbyran.OnlyFriends", "OnlyFriends");

    // Configure window attributes
    let window_attrs = dioxus_native::WindowAttributes::default()
        .with_title("OnlyFriends")
        .with_platform_attributes(Box::new(wayland_attrs))
        // Set the icon directly on platforms that support it.
        .with_window_icon(Some(icon.into()));

    dioxus_native::launch_cfg(App, vec![], vec![Box::new(window_attrs)]);
}

/// Root component.
#[component]
fn App() -> Element {
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
        let db_path = config::db_path().unwrap();
        spawn(async move {
            let store: Store = Arc::new(Mutex::new(SqliteStorage::open(&db_path).unwrap()));
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
    let db_path = config::db_path().unwrap();
    let storage: Store = Arc::new(Mutex::new(SqliteStorage::open(&db_path).unwrap()));
    let callback = use_callback(move |_| storage.clone());
    rsx! {
        pages::SetupPage {
            on_complete: move || {
                nav.push(Route::Feed {});
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
        pages::FriendsPage {
            on_copy_key: move |key| {
                // TODO: Consider making a context for the clipboard so that we can reuse it more easily.
                let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(key));
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
