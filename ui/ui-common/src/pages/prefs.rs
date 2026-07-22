use dioxus::prelude::*;

use crate::context;

/// View and edit app preferences.
#[component]
pub fn Prefs() -> Element {
    let account = context::use_app_account();
    let mut relay_url = use_signal(String::new);
    let mut editing = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut save_err = use_signal(String::new);
    let mut save_ok = use_signal(|| false);

    // Load preferences on mount.
    use_effect(move || {
        let account = account.read().as_ref().map(|a| a.clone());
        let Some(account) = account else { return };
        spawn(async move {
            let account = account.lock().await;
            let relay_config = account
                .store()
                .await
                .load_relay_config()
                .await
                .ok()
                .flatten();
            if let Some(relay_config) = relay_config {
                relay_url.set(relay_config.url);
            }
        });
    });

    let save = move |_| {
        let url = relay_url.read().trim().to_string();

        if url.is_empty() {
            save_err.set("URL cannot be empty.".to_string());
            return;
        }

        let account = account.read().as_ref().map(|a| a.clone());
        let Some(account) = account else { return };

        save_err.set(String::new());
        save_ok.set(false);
        saving.set(true);

        spawn(async move {
            let mut account = account.lock().await;
            match account.set_relay_url(url).await {
                Ok(()) => {
                    save_ok.set(true);
                    editing.set(false);
                }
                Err(e) => save_err.set(format!("Save failed: {e}")),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "page profile-page",
            h2 { "Preferences" }

            div { class: "card profile-card",
                div { class: "form-group",
                    label { "Relay URL" }
                    input {
                        r#type: "text",
                        class: "input",
                        value: "{relay_url}",
                        oninput: move |e| relay_url.set(e.value()),
                    }
                }
                if !save_err.read().is_empty() {
                    p { class: "error-msg", "{save_err}" }
                }
                div { class: "profile-actions",
                    button { class: "btn btn-primary", onclick: save,
                        if *saving.read() {
                            "Saving…"
                        } else {
                            "Save"
                        }
                    }
                }
            }
        }
    }
}
