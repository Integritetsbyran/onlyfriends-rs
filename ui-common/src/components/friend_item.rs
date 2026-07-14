use dioxus::prelude::*;

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// A single row in the friends list.
#[component]
pub fn FriendItem(friend: client_core::Friend) -> Element {
    let short_key = bytes_to_hex(&friend.public.sign_pub[..6]);

    rsx! {
        div { class: "card friend-item",
            div { class: "friend-info",
                span { class: "friend-nickname", "{friend.nickname}" }
                span { class: "friend-key", "{short_key}…" }
            }
        }
    }
}
