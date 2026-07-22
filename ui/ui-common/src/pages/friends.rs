use dioxus::{logger::tracing, prelude::*};

use crate::{components::FriendItem, context};

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Friends management page. Shows the user's own shareable public key, lists
/// current friends, and provides a form to add a new friend by pasting their
/// hex-encoded public identity bytes.
#[component]
pub fn FriendsPage() -> Element {
    let account = context::use_app_account();
    let mut friends = use_signal(Vec::<client_core::Friend>::new);
    let mut own_hex = use_signal(String::new);

    // Load friends + own key on mount.
    use_effect(move || {
        let Some(arc) = account.read().as_ref().map(|a| a.clone()) else {
            return;
        };

        spawn(async move {
            let result: client_core::Result<()> = async {
                let acc = arc.lock().await;

                let pub_id = acc.identity.public();
                let hex = bytes_to_hex(&pub_id.to_bytes());
                own_hex.set(hex);

                let list = acc.store().await.load_friends().await?;
                friends.set(list);
                Ok(())
            }
            .await;

            if let Err(e) = result {
                tracing::warn!("Failed to load friends: {e}");
            };
        });
    });

    // Add-friend form state.
    let mut their_key_hex = use_signal(String::new);
    let mut nickname_input = use_signal(String::new);
    let mut add_err = use_signal(String::new);
    let mut adding = use_signal(|| false);

    let add_friend = move |_| {
        let hex = their_key_hex.read().trim().to_string();
        let nick = nickname_input.read().trim().to_string();

        if hex.is_empty() {
            add_err.set("Paste their public key first.".to_string());
            return;
        }
        if nick.is_empty() {
            add_err.set("Nickname is required.".to_string());
            return;
        }

        let Some(key_bytes) = hex_to_bytes(&hex) else {
            add_err.set("Invalid key: not valid hex.".to_string());
            return;
        };

        let Ok(their_pub) = client_core::PublicIdentity::try_from(&key_bytes[..]) else {
            add_err.set("Invalid key: wrong length or format.".to_string());
            return;
        };

        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else {
            add_err.set("Not logged in.".to_string());
            return;
        };

        add_err.set(String::new());
        adding.set(true);

        spawn(async move {
            let mut acc = arc.lock().await;

            if let Err(e) = acc.add_friend(&their_pub, &nick).await {
                add_err.set(format!("Failed to add friend: {e}"));
                adding.set(false);
                return;
            }

            let result: client_core::Result<()> = async {
                their_key_hex.set(String::new());
                nickname_input.set(String::new());

                let list = acc.store().await.load_friends().await?;
                friends.set(list);
                Ok(())
            }
            .await;

            if let Err(e) = result {
                add_err.set(format!("Failed to get new friends list: {e:?}"));
            }

            adding.set(false);
        });
    };

    rsx! {
        div { class: "page friends-page",
            h2 { "Friends" }

            // Own identity card
            div { class: "card own-key-card",
                h3 { "Your public key" }
                p { class: "hint", "Share this with friends so they can add you." }
                input {
                    r#type: "text",
                    class: "input key-input",
                    readonly: true,
                    value: "{own_hex}",
                }
            }

            // Add friend form
            div { class: "card add-friend-card",
                h3 { "Add a friend" }
                div { class: "form-group",
                    label { "Their public key" }
                    input {
                        r#type: "text",
                        class: "input",
                        placeholder: "Paste hex key…",
                        value: "{their_key_hex}",
                        oninput: move |e| their_key_hex.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { "Nickname" }
                    input {
                        r#type: "text",
                        class: "input",
                        placeholder: "e.g. Alice",
                        value: "{nickname_input}",
                        oninput: move |e| nickname_input.set(e.value()),
                    }
                }
                if !add_err.read().is_empty() {
                    p { class: "error-msg", "{add_err}" }
                }
                button {
                    class: "btn btn-primary",
                    disabled: *adding.read(),
                    onclick: add_friend,
                    if *adding.read() {
                        "Adding…"
                    } else {
                        "Add friend"
                    }
                }
            }

            // Friends list
            div { class: "friends-list",
                if friends.read().is_empty() {
                    p { class: "empty-state", "No friends yet. Add one above!" }
                }
                for friend in friends.read().iter().cloned() {
                    FriendItem { friend }
                }
            }
        }
    }
}
