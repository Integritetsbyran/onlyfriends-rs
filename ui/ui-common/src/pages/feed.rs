use dioxus::prelude::*;

use crate::{
    components::{Modal, NewPostForm, PostCard},
    context,
};
use keystone::post::PostId;
use std::sync::Arc;
use std::collections::{BTreeMap, HashSet, HashMap};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
struct PostKey {
    // sort by timestamp and then id
    timestamp: u64, // TODO
    id: PostId,
}

impl PostKey {
    pub fn of(post: &client_core::FeedPost) -> Self {
        Self {
            timestamp: post.created_at,
            id: post.id,
        }
    }
}

type Posts = BTreeMap<PostKey, Arc<client_core::FeedPost>>;

fn sync_feed(
    feed: &[client_core::FeedPost],
    store: &mut Store<Posts>
) {
    let new: HashMap<_, _> = feed.into_iter().map(|p| (PostKey::of(p), p)).collect();
    let new_keys: HashSet<_> = new.keys().copied().collect();
    let old_keys: HashSet<_> = store().keys().copied().collect();
    let added = new_keys.difference(&old_keys);
    let removed = old_keys.difference(&new_keys);
    dbg!(&added);
    dbg!(&removed);

    for &key in added {
        let &post = new.get(&key).expect("key exists");
        store.insert(key, Arc::new(post.clone()));
    }

    for key in removed {
        store.remove(key);
    }
}

/// Main feed page. Auto-syncs all friends' mailboxes on mount, then displays
/// posts in reverse-chronological order.
#[component]
pub fn FeedPage() -> Element {
    let account = context::use_app_account();
    let mut posts: Store<Posts> = use_store(BTreeMap::new);
    let mut syncing = use_signal(|| false);
    let mut sync_err = use_signal(String::new);

    // Auto-sync + load on mount.
    use_effect(move || {
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        if let Some(arc) = acc_opt {
            syncing.set(true);
            sync_err.set(String::new());
            spawn(async move {
                let mut acc = arc.lock().await;
                if let Err(e) = acc.sync().await {
                    sync_err.set(format!("Sync error: {e}"));
                }
                match acc.load_feed() {
                    Ok(feed) => {
                        let feed = feed.into_iter()
                            .map(Arc::new)
                            .map(|post| (PostKey::of(&post), post))
                            .collect();
                        posts.set(feed);
                    }
                    Err(e) => sync_err.set(format!("Load error: {e}")),
                }
                syncing.set(false);
            });
        }
    });

    // Callback given to NewPostForm so it can reload the feed after posting.
    let refresh = move |_| {
        let acc_opt = account.read().as_ref().map(|a| a.clone());
        if let Some(arc) = acc_opt {
            spawn(async move {
                let mut acc = arc.lock().await;
                if let Ok(feed) = acc.load_feed() {
                    sync_feed(&feed, &mut posts);
                }
            });
        }
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
                for (key, post) in posts.iter().rev() {
                    // TODO: timestamp also? does dioxus care?
                    {let id = u128::from_be_bytes(key.id.0);
                    rsx! { PostCard { key: "{id}", post } }
                    }
                }
            }
        }
    }
}
