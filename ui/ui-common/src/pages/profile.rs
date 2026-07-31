use dioxus::prelude::*;

use crate::context;

/// Profile page — view and edit the user's own display name and bio.
#[component]
pub fn ProfilePage() -> Element {
    let account = context::use_app_account();
    let mut display_name = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut editing = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut save_err = use_signal(String::new);
    let mut save_ok = use_signal(|| false);

    // Load own profile on mount.
    use_effect(move || {
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        if let Some(arc) = acc_opt {
            spawn(async move {
                let acc = arc.lock().await;
                let sign_pub = acc.identity.public().sign_pub;
                if let Ok(Ok(Some(profile))) = acc.store().map(|mut s| s.load_profile(&sign_pub)) {
                    display_name.set(profile.display_name.clone());
                    bio.set(profile.bio.clone());
                }
            });
        }
    });

    let save = move |_| {
        let name = display_name.read().trim().to_string();
        let bio_text = bio.read().trim().to_string();

        if name.is_empty() {
            save_err.set("Display name cannot be empty.".to_string());
            return;
        }

        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else { return };

        save_err.set(String::new());
        save_ok.set(false);
        saving.set(true);

        spawn(async move {
            let acc = arc.lock().await;
            match acc.set_profile(&name, &bio_text).await {
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
            h2 { "Profile" }

            div { class: "card profile-card",
                if *editing.read() {
                    // Edit mode
                    div { class: "form-group",
                        label { "Display name" }
                        input {
                            r#type: "text",
                            class: "input",
                            value: "{display_name}",
                            oninput: move |e| display_name.set(e.value()),
                        }
                    }
                    div { class: "form-group",
                        label { "Bio" }
                        textarea {
                            class: "input",
                            value: "{bio}",
                            oninput: move |e| bio.set(e.value()),
                        }
                    }
                    if !save_err.read().is_empty() {
                        p { class: "error-msg", "{save_err}" }
                    }
                    div { class: "profile-actions",
                        button {
                            class: "btn btn-primary",
                            disabled: *saving.read(),
                            onclick: save,
                            if *saving.read() {
                                "Saving…"
                            } else {
                                "Save"
                            }
                        }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |_| {
                                editing.set(false);
                                save_err.set(String::new());
                            },
                            "Cancel"
                        }
                    }
                } else {
                    // View mode
                    div { class: "profile-view",
                        p { class: "profile-name", "{display_name}" }
                        if !bio.read().is_empty() {
                            p { class: "profile-bio", "{bio}" }
                        }
                        if *save_ok.read() {
                            p { class: "success-msg", "Profile updated." }
                        }
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                editing.set(true);
                                save_ok.set(false);
                            },
                            "Edit"
                        }
                    }
                }
            }
        }
    }
}
