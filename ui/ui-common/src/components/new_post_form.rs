use dioxus::prelude::*;

use crate::context;

/// Inline form at the top of the feed for composing a new post.
#[component]
pub fn NewPostForm(on_posted: EventHandler<()>) -> Element {
    let account = context::use_app_account();
    let mut body = use_signal(String::new);
    let mut posting = use_signal(|| false);
    let mut err = use_signal(String::new);

    let mut do_post = move || {
        let text = body.read().trim().to_string();
        if text.is_empty() {
            return;
        }
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else { return };

        err.set(String::new());
        posting.set(true);
        spawn(async move {
            let mut acc = arc.lock().await;
            match acc.send_text_post(&text).await {
                Ok(_) => {
                    body.set(String::new());
                    on_posted.call(());
                }
                Err(e) => err.set(format!("Post failed: {e}")),
            }
            posting.set(false);
        });
    };

    rsx! {
        div { class: "card new-post-form",
            textarea {
                class: "input post-textarea",
                placeholder: "What's on your mind?",
                value: "{body}",
                oninput: move |e| body.set(e.value()),
                onkeydown: move |e| {
                    // Ctrl/Cmd + Enter to submit
                    if e.key() == Key::Enter && e.modifiers().ctrl() {
                        do_post();
                    }
                },
            }
            div { class: "new-post-actions",
                if !err.read().is_empty() {
                    span { class: "error-inline", "{err}" }
                }
                button {
                    class: "btn btn-primary",
                    disabled: *posting.read(),
                    onclick: move |_| do_post(),
                    if *posting.read() {
                        "Posting…"
                    } else {
                        "Post"
                    }
                }
            }
        }
    }
}
