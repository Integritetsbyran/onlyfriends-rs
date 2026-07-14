use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;

use crate::components::{CommentList, ReactionBar};
use crate::context;

fn format_age(ts: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(ts);
    if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// A single post card in the feed.
#[component]
pub fn PostCard(post: client_core::FeedPost) -> Element {
    let account = context::use_app_account();
    let mut author_name = use_signal(|| bytes_to_hex(&post.author[..8]));
    let mut show_comments = use_signal(|| false);

    // Try to resolve author display name from stored profiles.
    let author_key = post.author;
    use_effect(move || {
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        if let Some(arc) = acc_opt {
            spawn(async move {
                let acc = arc.lock().await;
                if let Ok(Some(profile)) = acc.storage.load_profile(&author_key) {
                    author_name.set(profile.display_name.clone());
                } else {
                    // Check if it's our own post.
                    let own = acc.identity.public().sign_pub;
                    if own == author_key {
                        if let Ok(Some(p)) = acc.storage.load_profile(&own) {
                            author_name.set(format!("{} (you)", p.display_name));
                        } else {
                            author_name.set("You".to_string());
                        }
                    }
                }
            });
        }
    });

    let age = format_age(post.created_at);
    let comment_count = post.comments.len();

    rsx! {
        div { class: "card post-card",
            div { class: "post-header",
                span { class: "post-author", "{author_name}" }
                span { class: "post-age", "{age}" }
            }

            p { class: "post-body", "{post.body}" }

            ReactionBar {
                post_id: post.id,
                post_author: post.author,
                reactions: post.reactions.clone(),
            }

            button {
                class: "btn-ghost comment-toggle",
                onclick: move |_| show_comments.toggle(),
                if comment_count == 0 {
                    "Add comment"
                } else if *show_comments.read() {
                    "Hide comments ({comment_count})"
                } else {
                    "Show comments ({comment_count})"
                }
            }

            if *show_comments.read() {
                CommentList {
                    post_id: post.id,
                    post_author: post.author,
                    comments: post.comments.clone(),
                }
            }
        }
    }
}
