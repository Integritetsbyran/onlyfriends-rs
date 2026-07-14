use std::sync::Arc;

use dioxus::prelude::*;
use tokio::sync::Mutex;

use crate::{config, context};

/// First-run onboarding screen. Collects a relay URL, display name, and bio,
/// opens (or creates) the local database, persists the account in context,
/// and calls `on_complete` so the caller can navigate away.
#[component]
pub fn SetupPage(on_complete: EventHandler<()>) -> Element {
    let mut relay_url = use_signal(|| "http://localhost:3000".to_string());
    let mut display_name = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut error_msg = use_signal(String::new);
    let mut submitting = use_signal(|| false);

    let mut account = context::use_app_account();

    let submit = move |_| {
        let relay = relay_url.read().trim().to_string();
        let name = display_name.read().trim().to_string();
        let bio_text = bio.read().trim().to_string();

        if relay.is_empty() {
            error_msg.set("Relay URL is required.".to_string());
            return;
        }
        if name.is_empty() {
            error_msg.set("Display name is required.".to_string());
            return;
        }

        error_msg.set(String::new());
        submitting.set(true);

        spawn(async move {
            match client_core::Account::open(&config::db_path(), &relay) {
                Ok(acc) => {
                    // Best-effort: set the profile; ignore errors here — user can update later.
                    let _ = acc.set_profile(&name, &bio_text).await;
                    account.set(Some(Arc::new(Mutex::new(acc))));
                    on_complete.call(());
                }
                Err(e) => {
                    error_msg.set(format!("Failed to open account: {e}"));
                    submitting.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "setup-page",
            div { class: "setup-card",
                h1 { class: "setup-title", "OnlyFriends" }
                p { class: "setup-subtitle",
                    "Private, encrypted social with your friends."
                }

                div { class: "form-group",
                    label { "Relay server URL" }
                    input {
                        r#type: "text",
                        class: "input",
                        placeholder: "http://localhost:3000",
                        value: "{relay_url}",
                        oninput: move |e| relay_url.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label { "Display name" }
                    input {
                        r#type: "text",
                        class: "input",
                        placeholder: "Your name",
                        value: "{display_name}",
                        oninput: move |e| display_name.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label { "Bio " span { class: "optional", "(optional)" } }
                    textarea {
                        class: "input",
                        placeholder: "A short bio…",
                        value: "{bio}",
                        oninput: move |e| bio.set(e.value()),
                    }
                }

                if !error_msg.read().is_empty() {
                    p { class: "error-msg", "{error_msg}" }
                }

                button {
                    class: "btn btn-primary",
                    disabled: *submitting.read(),
                    onclick: submit,
                    if *submitting.read() { "Setting up…" } else { "Get started" }
                }
            }
        }
    }
}
