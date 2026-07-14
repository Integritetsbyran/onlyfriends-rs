use dioxus::prelude::*;
use keystone::post::PostId;

use crate::context;

/// Emoji reaction bar displayed under a post.
#[component]
pub fn ReactionBar(
    post_id: PostId,
    post_author: [u8; 32],
    reactions: Vec<client_core::FeedReaction>,
) -> Element {
    let account = context::use_app_account();
    let mut emoji_input = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut err = use_signal(String::new);

    // Group reactions: emoji → count
    let mut grouped: Vec<(String, usize)> = {
        let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for r in &reactions {
            *map.entry(r.emoji.clone()).or_insert(0) += 1;
        }
        let mut v: Vec<_> = map.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    };
    grouped.sort_by(|a, b| b.1.cmp(&a.1));

    let mut do_react = move || {
        let emoji = emoji_input.read().trim().to_string();
        if emoji.is_empty() {
            return;
        }
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else { return };

        err.set(String::new());
        sending.set(true);
        spawn(async move {
            let acc = arc.lock().await;
            match acc.react(post_id, &post_author, &emoji).await {
                Ok(()) => emoji_input.set(String::new()),
                Err(e) => err.set(format!("{e}")),
            }
            sending.set(false);
        });
    };

    rsx! {
        div { class: "reaction-bar",
            // Existing reactions
            for (emoji, count) in grouped.iter() {
                span { class: "reaction-pill",
                    "{emoji} {count}"
                }
            }

            // Inline add-reaction input
            input {
                r#type: "text",
                class: "input reaction-input",
                placeholder: "React…",
                value: "{emoji_input}",
                oninput: move |e| emoji_input.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        do_react();
                    }
                },
            }
            button {
                class: "btn-ghost",
                disabled: *sending.read(),
                onclick: move |_| do_react(),
                "+"
            }

            if !err.read().is_empty() {
                span { class: "error-inline", "{err}" }
            }
        }
    }
}
