use dioxus::prelude::*;

/// A single row in the friends list.
#[component]
pub fn FriendItem(friend: client_core::Friend) -> Element {
    let short_key = friend.public.sign_pub.to_short_hex();

    rsx! {
        div { class: "card friend-item",
            div { class: "friend-info",
                span { class: "friend-nickname", "{friend.nickname}" }
                span { class: "friend-key", "{short_key}…" }
            }
        }
    }
}
