use dioxus::prelude::*;

use crate::{
    components::{Modal, NewPostForm, PostCard},
    context,
};
use std::sync::Arc;

/// Main feed page. Auto-syncs all friends' mailboxes on mount, then displays
/// posts in reverse-chronological order.
#[component]
pub fn FeedPage() -> Element {
    let account = context::use_app_account();
    let mut posts = use_signal(Vec::<Arc<client_core::FeedPost>>::new);
    let mut syncing = use_signal(|| false);
    let mut sync_err = use_signal(String::new);

    // Auto-sync + load on mount.
    use_effect(move || {
        let Some(arc) = account.read().as_ref().map(|a| a.clone()) else {
            return;
        };

        syncing.set(true);
        sync_err.set(String::new());
        spawn(async move {
            let mut acc = arc.lock().await;

            if let Err(e) = acc.sync().await {
                sync_err.set(format!("Sync error: {e}"));
            }

            match acc.load_feed().await {
                Ok(feed) => {
                    let feed = feed.into_iter().map(Arc::new).collect();
                    posts.set(feed);
                }
                Err(e) => sync_err.set(format!("Load error: {e}")),
            }
            syncing.set(false);
        });
    });

    // Callback given to NewPostForm so it can reload the feed after posting.
    let refresh = move |_| {
        let Some(arc) = account.read().as_ref().map(|a| a.clone()) else {
            return;
        };

        spawn(async move {
            let mut acc = arc.lock().await;
            if let Ok(feed) = acc.load_feed().await {
                let feed = feed.into_iter().map(Arc::new).collect();
                posts.set(feed);
            }
        });
    };

    rsx! {
        Modal {}
        div { class: "page feed-page",
            div { class: "feed-header",
                h2 { "Feed" }
                if *syncing.read() {
                    span { class: "sync-badge", "Syncing…" }
                }
            }

            if !sync_err.read().is_empty() {
                p { class: "error-msg", "{sync_err}" }
            }

            NewPostForm { on_posted: refresh }

            div { class: "post-list",
                if posts.read().is_empty() && !*syncing.read() {
                    p { class: "empty-state",
                        "Nothing here yet. Add some friends and posts will appear."
                    }
                }
                for post in posts.read().iter().cloned() {
                    PostCard { post }
                }
            }
        }
    }
}
