use dioxus::prelude::*;

use crate::context::{AppAccount, ModalContent};

#[component]
pub fn AppRoot(
    on_feed: EventHandler<()>,
    on_friends: EventHandler<()>,
    on_profile: EventHandler<()>,
    on_prefs: EventHandler<()>,
    account: AppAccount,
    children: Element,
) -> Element {
    // Take ownership of account and wrap it in a signal so that it can be provided to descendants.
    let account_signal = use_signal(|| account.clone());
    use_context_provider(|| account_signal);
    use_context_provider(|| Signal::new(None::<ModalContent>));

    rsx! {
        div { class: "app-root",
            nav { class: "top-nav",
                span { class: "app-title", "OnlyFriends" }
                a { class: "nav-tab", onclick: move |_| on_feed.call(()), "Feed" }
                a { class: "nav-tab", onclick: move |_| on_friends.call(()), "Friends" }
                a { class: "nav-tab", onclick: move |_| on_profile.call(()), "Profile" }
                a { class: "nav-tab", onclick: move |_| on_prefs.call(()), "⚙" }
            }
            {children}
        }
    }
}
