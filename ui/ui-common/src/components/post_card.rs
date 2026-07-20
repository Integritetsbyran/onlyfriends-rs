use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use dioxus::prelude::*;
use keystone::media::Media;

use crate::components::{CommentList, ReactionBar};
use crate::context::{self, ModalContent};

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

/// Convert a [`Media`] into an web-compatible data-blob.
///
/// Expensive and ridiculous, but this seems to be the only way to load image bytes into dioxus.
/// Note that it takes also a noticable amount of time for dioxus to **decode** the base64 image.
fn media_to_data_uri(media: &Media) -> Arc<str> {
    let b64 = STANDARD.encode(&media.bytes);
    // TODO: filter unknown mime types?
    let mime = media.mime.as_ref();
    format!("data:{mime};base64,{b64}").into()
}

/// A single post card in the feed.
#[component]
pub fn PostCard(post: ReadSignal<Arc<client_core::FeedPost>>) -> Element {
    let post = post(); // TODO: make reactive
    let account = context::use_app_account();
    let mut modal = context::use_modal();
    let mut author_name = use_signal(|| bytes_to_hex(&post.author[..8]));
    let mut show_comments = use_signal(|| false);
    let has_media = !post.content.media.is_empty();
    let mut media = use_signal::<Option<Arc<[_]>>>(|| None);

    // Try to resolve author display name from stored profiles.
    let author_key = post.author;
    use_effect(move || {
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        let Some(arc) = acc_opt else { return };
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
    });

    let age = format_age(post.created_at);
    let comment_count = post.comments.len();

    // Base64-encode media. This is expensive, so only do it once.
    if has_media && media().is_none() {
        // TODO: don't convert images in the render loop
        let m: Vec<_> = post.content.media.iter().map(media_to_data_uri).collect();
        let m: Arc<_> = m.into_boxed_slice().into();
        media.set(Some(m));
    }

    rsx! {
        div { class: "card post-card",
            div { class: "post-header",
                span { class: "post-author", "{author_name}" }
                span { class: "post-age", "{age}" }
            }

            p { class: "post-body", "{post.content.body}" }

            if let Some(media) = media() {
                div { class: "post-images",
                    {media.iter().map(|media| {
                        let media = Arc::clone(media);
                        rsx! {
                            img {
                                src: "{media}",
                                onclick: move |_| {
                                    modal.set(Some(ModalContent(rsx! {
                                        img {
                                            class: "post-image-viewer",
                                            src: "{media}",
                                        }
                                    })));
                                }
                            }
                        }
                    })}
                }
            }

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
