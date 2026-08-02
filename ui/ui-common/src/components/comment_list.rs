use crate::context;
use dioxus::prelude::*;
use keystone::envelope::LetterId;
use keystone::identity::SigningPublicKey;

/// Expandable comment thread under a post.
#[component]
pub fn CommentList(
    letter_id: LetterId,
    post_author: SigningPublicKey,
    comments: Vec<client_core::FeedComment>,
) -> Element {
    let account = context::use_app_account();
    let mut new_comment = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut err = use_signal(String::new);

    let mut do_send = move || {
        let text = new_comment.read().trim().to_string();
        if text.is_empty() {
            return;
        }
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else { return };

        err.set(String::new());
        sending.set(true);
        spawn(async move {
            let acc = arc.lock().await;
            match acc.comment(letter_id, &post_author, &text).await {
                Ok(()) => new_comment.set(String::new()),
                Err(e) => err.set(format!("{e}")),
            }
            sending.set(false);
        });
    };

    rsx! {
        div { class: "comment-list",
            for comment in comments.iter() {
                div { class: "comment",
                    span { class: "comment-author", "{comment.author.to_short_hex()}…" }
                    span { class: "comment-text", "{comment.text}" }
                }
            }

            // New comment row
            div { class: "comment-form",
                input {
                    r#type: "text",
                    class: "input",
                    placeholder: "Write a comment…",
                    value: "{new_comment}",
                    oninput: move |e| new_comment.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            do_send();
                        }
                    },
                }
                button {
                    class: "btn btn-secondary",
                    disabled: *sending.read(),
                    onclick: move |_| do_send(),
                    if *sending.read() {
                        "…"
                    } else {
                        "Send"
                    }
                }
            }

            if !err.read().is_empty() {
                p { class: "error-msg", "{err}" }
            }
        }
    }
}
