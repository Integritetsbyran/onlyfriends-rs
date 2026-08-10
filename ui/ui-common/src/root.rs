use dioxus::prelude::*;

use crate::context;

#[component]
pub fn AppRoot(
    on_feed: EventHandler<()>,
    on_friends: EventHandler<()>,
    on_profile: EventHandler<()>,
    on_prefs: EventHandler<()>,
    children: Element,
) -> Element {
    let account = context::use_app_account();

    rsx! {
        div { class: "app-root",
            if account.read().is_some() {
                nav { class: "top-nav",
                    span { class: "app-title", "OnlyFriends" }
                    a { class: "nav-tab", onclick: move |_| on_feed.call(()), "Feed" }
                    a { class: "nav-tab", onclick: move |_| on_friends.call(()), "Friends" }
                    a { class: "nav-tab", onclick: move |_| on_profile.call(()), "Profile" }
                    a { class: "nav-tab", onclick: move |_| on_prefs.call(()), "⚙" }
                }
            }
            {children}
        }
    }
}
