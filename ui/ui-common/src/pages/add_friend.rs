use dioxus::prelude::*;

use crate::{context, hex_util::hex_to_bytes};

/// Confirmation screen shown after following an "add friend" deep link
/// (e.g. from a scanned QR code / universal link).

#[component]
pub fn AddFriendPage(code: String, on_added: EventHandler<()>) -> Element {
    let account = context::use_app_account();
    let mut nickname_input = use_signal(String::new);
    let mut add_err = use_signal(String::new);
    let mut adding = use_signal(|| false);

    let parsed = hex_to_bytes(code.trim())
        .and_then(|bytes| client_core::PublicIdentity::try_from(&bytes[..]).ok());

    let Some(their_pub) = parsed else {
        return rsx! {
            div { class: "page add-friend-page",
                h2 { "Add friend" }
                p { class: "error-msg", "This link doesn't contain a valid public key." }
            }
        };
    };

    // Short fingerprint for display, so the user has something to eyeball
    // before trusting the link.
    let fingerprint = code.chars().take(16).collect::<String>().to_uppercase();

    let confirm = move |_| {
        let nick = nickname_input.read().trim().to_string();
        if nick.is_empty() {
            add_err.set("Nickname is required.".to_string());
            return;
        }

        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else {
            add_err.set("Not logged in.".to_string());
            return;
        };

        let their_pub = their_pub.clone();
        add_err.set(String::new());
        adding.set(true);

        spawn(async move {
            let mut acc = arc.lock().await;
            match acc.add_friend(&their_pub, &nick).await {
                Ok(_) => on_added.call(()),
                Err(e) => add_err.set(format!("Failed to add friend: {e}")),
            }
            adding.set(false);
        });
    };

    rsx! {
        div { class: "page add-friend-page",
            h2 { "Add friend" }
            div { class: "card add-friend-card",
                p { class: "hint", "You're about to add a new friend." }
                p { class: "fingerprint", "Key: {fingerprint}…" }

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
                    onclick: confirm,
                    if *adding.read() {
                        "Adding…"
                    } else {
                        "Add friend"
                    }
                }
            }
        }
    }
}
