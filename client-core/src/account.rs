use keystone::{
    ResponseBody::{Comment, Reaction},
    identity::SigningPublicKey,
    post::{PostContent, PostId},
};
use std::sync::{Arc, Mutex};
use storage_common::{storage::Storage, types::stored_response::ResponseKind};

use crate::{ClientError, RelayClient, epoch_now, mailbox_address, my_direction};

pub type Store = Arc<Mutex<dyn Storage>>;

pub struct Account {
    storage: Store,
    pub identity: keystone::Identity,
    pub relay: RelayClient,
}

#[derive(Debug)]
pub struct SyncResult {
    pub new_posts: Vec<PostContent>,
    pub updated_profiles: Vec<keystone::Profile>,
    pub new_responses: Vec<keystone::response::ResponseInner>,
}

impl SyncResult {
    pub fn new() -> SyncResult {
        SyncResult {
            new_posts: Vec::new(),
            updated_profiles: Vec::new(),
            new_responses: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: SyncResult) {
        self.new_posts.extend(other.new_posts);
        self.updated_profiles.extend(other.updated_profiles);
        self.new_responses.extend(other.new_responses);
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedPost {
    pub id: PostId,
    pub author: SigningPublicKey,
    pub created_at: u64,
    pub content: PostContent,
    pub reactions: Vec<FeedReaction>,
    pub comments: Vec<FeedComment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedReaction {
    pub author: SigningPublicKey,
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedComment {
    pub author: SigningPublicKey,
    pub text: String, // already decrypted
}

impl Account {
    pub fn store(&self) -> crate::Result<std::sync::MutexGuard<'_, dyn Storage + 'static>> {
        self.storage
            .lock()
            .map_err(|e| ClientError::PoisonError(e.to_string()))
    }

    pub fn open(storage: Store, relay_url: &str) -> crate::Result<Option<Self>> {
        let identity = storage
            .lock()
            .map_err(|e| ClientError::PoisonError(e.to_string()))?
            .load_identity()?;
        let Some(identity) = identity else {
            return Ok(None);
        };
        let relay = RelayClient::new(relay_url);
        Ok(Some(Account {
            storage,
            identity,
            relay,
        }))
    }

    pub fn create_new(storage: Store, relay_url: &str) -> crate::Result<Self> {
        let identity = load_or_create_identity(&storage)?;
        let relay = RelayClient::new(relay_url);
        Ok(Account {
            storage,
            identity,
            relay,
        })
    }

    pub async fn add_friend(
        &mut self,
        their: &keystone::PublicIdentity,
        nickname: &str,
    ) -> crate::Result<keystone::Friend> {
        let friend = keystone::friend::add_friend(&self.identity, their, nickname);
        self.store()?.save_friend(&friend)?;
        self.send_my_profile_to(&friend).await?;
        Ok(friend)
    }

    /// Send `post` to all friends.
    ///
    /// Returns the post id, or `None` if you have no friends.
    pub async fn send_text_post(
        &mut self,
        body: impl Into<String>,
    ) -> crate::Result<Option<PostId>> {
        self.send_post(&PostContent::from_body(body)).await
    }

    /// Send `post` to all friends.
    ///
    /// Returns the post id, or `None` if you have no friends.
    pub async fn send_post(&mut self, post: &PostContent) -> crate::Result<Option<PostId>> {
        let friends = self.store()?.load_friends()?;
        let recipients: Vec<_> = friends.iter().map(|f| f.public.clone()).collect();

        // Always include self as a recipient so `create_post` always returns at
        // least one SealedPost.  That gives us the Post metadata (id, timestamp,
        // signature) needed to persist the post locally regardless of whether the
        // user has any friends yet.
        let mut recipients_with_self = recipients.clone();
        recipients_with_self.push(self.identity.public());
        let posts = keystone::post::seal_post(&self.identity, post, &recipients_with_self);

        let post_id = posts.first().map(|p| p.post.id);
        if let Some(first) = posts.first() {
            self.store()?.save_post(&first.post, post)?;
        }

        // Send only to actual friends; the trailing self-sealed post is unused.
        for (friend, post) in friends.iter().zip(posts.iter()) {
            let envelope: keystone::Envelope = keystone::Envelope::Post(post.clone());
            let epoch = epoch_now(60 * 60 * 24);
            let addr = mailbox_address(
                &friend.pairwise_root,
                my_direction(&self.identity.public(), &friend.public),
                epoch,
            );

            self.relay.post_item(&addr, &envelope).await?;
        }
        Ok(post_id)
    }

    pub async fn process_mailbox(
        &mut self,
        friend: &keystone::Friend,
    ) -> crate::Result<SyncResult> {
        let direction = my_direction(&friend.public, &self.identity.public());
        let current = epoch_now(60 * 60 * 24);
        let start = current.saturating_sub(7);

        let mut sync_results = SyncResult::new();

        for e in start..=current {
            let addr = mailbox_address(&friend.pairwise_root, direction, e);
            let after = self
                .store()?
                .get_cursor(&friend.public.sign_pub, direction, e)?;
            let items = self.relay.get_items(&addr, after).await?;

            for item in &items {
                let Ok(envelope) = postcard::from_bytes::<keystone::Envelope>(item) else {
                    continue;
                };

                match envelope {
                    keystone::Envelope::Post(sealed_post) => {
                        let Ok(post) =
                            keystone::post::open_post(&self.identity, &friend.public, &sealed_post)
                        else {
                            continue;
                        };
                        if self.store()?.save_post(&sealed_post.post, &post)? {
                            sync_results.new_posts.push(post);
                        }
                    }
                    keystone::Envelope::Profile(sealed) => {
                        let Ok(profile_bytes) =
                            keystone::SealedBox::open(&self.identity.dh_secret(), &sealed)
                        else {
                            continue;
                        };

                        let Ok(profile) = postcard::from_bytes::<keystone::Profile>(&profile_bytes)
                        else {
                            continue;
                        };

                        if keystone::profile::verify_profile(&profile).is_err() {
                            continue;
                        }

                        let existing = self.store()?.load_profile(&profile.owner)?;
                        let is_newer = existing.is_none_or(|old| profile.version > old.version);
                        if is_newer {
                            self.store()?.save_profile(&profile)?;
                            sync_results.updated_profiles.push(profile);
                        }
                    }
                    keystone::Envelope::Response(sealed_box) => {
                        let Ok(rb) =
                            keystone::response::open_and_vouch(&self.identity, &sealed_box)
                        else {
                            continue;
                        };

                        let friends = self.store()?.load_friends()?;
                        for f in friends {
                            let rb_bytes = postcard::to_allocvec(&rb)?;
                            let resealed = keystone::SealedBox::seal(&f.public.dh_pub, &rb_bytes);
                            self.post_envelope(&f, keystone::Envelope::Rebroadcast(resealed))
                                .await?;
                        }
                        sync_results.new_responses.push(rb.inner.clone());
                        self.save_response(rb)?;
                    }
                    keystone::Envelope::Rebroadcast(sealed_box) => {
                        let Ok(rb) = keystone::response::open_rebroadcast(
                            &self.identity,
                            &friend.public.sign_pub,
                            &sealed_box,
                        ) else {
                            continue;
                        };

                        sync_results.new_responses.push(rb.inner.clone());
                        self.save_response(rb)?;
                    }
                }
            }
            if !items.is_empty() {
                self.store()?.set_cursor(
                    &friend.public.sign_pub,
                    direction,
                    e,
                    after + items.len(),
                )?;
            }
        }

        Ok(sync_results)
    }

    fn save_response(&self, rb: keystone::response::ResponseRebroadcast) -> crate::Result<()> {
        let post_id = rb.inner.post_id;
        self.store()?.save_response(&post_id, &rb.inner.into())?;
        Ok(())
    }

    /// Set/update my own profile and push it to every friend.
    pub async fn set_profile(&self, display_name: &str, bio: &str) -> crate::Result<()> {
        let old_profile = self
            .store()?
            .load_profile(&self.identity.public().sign_pub)?;

        let new_profile = keystone::profile::create_profile(
            &self.identity,
            display_name,
            bio,
            old_profile.map_or(0, |p| p.version + 1),
        );
        self.store()?.save_profile(&new_profile)?;

        let friends = self.store()?.load_friends()?;
        for friend in friends.iter() {
            self.send_my_profile_to(friend).await?
        }

        Ok(())
    }

    async fn post_envelope(
        &self,
        friend: &keystone::Friend,
        envelope: keystone::Envelope,
    ) -> crate::Result<()> {
        let addr = mailbox_address(
            &friend.pairwise_root,
            my_direction(&self.identity.public(), &friend.public),
            epoch_now(60 * 60 * 24),
        );
        self.relay.post_item(&addr, &envelope).await?;
        Ok(())
    }

    async fn send_my_profile_to(&self, friend: &keystone::Friend) -> crate::Result<()> {
        let Some(profile) = self
            .store()?
            .load_profile(&self.identity.public().sign_pub)?
        else {
            return Ok(());
        };
        let profile_bytes = postcard::to_allocvec(&profile)?;
        let envelope = keystone::Envelope::Profile(keystone::SealedBox::seal(
            &friend.public.dh_pub,
            &profile_bytes,
        ));

        self.post_envelope(friend, envelope).await?;
        Ok(())
    }

    pub async fn react(
        &self,
        post_id: PostId,
        post_author: &SigningPublicKey,
        emoji: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self.store()?.load_friend_by_sign_pub(post_author)? else {
            return Err(ClientError::NotFriendError("react to a post"));
        };
        let inner = keystone::response::create_response(
            &self.identity,
            post_id,
            Reaction {
                emoji: emoji.to_string(),
            },
        );

        let bytes = postcard::to_allocvec(&inner)?;
        let sealed = keystone::SealedBox::seal(&owner.public.dh_pub, &bytes);
        self.post_envelope(&owner, keystone::Envelope::Response(sealed))
            .await
    }

    pub async fn comment(
        &self,
        post_id: PostId,
        post_author: &SigningPublicKey,
        text: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self.store()?.load_friend_by_sign_pub(post_author)? else {
            return Err(ClientError::NotFriendError("comment on a post"));
        };
        let inner = keystone::response::create_response(
            &self.identity,
            post_id,
            Comment {
                text: text.to_string(),
            },
        );

        let bytes = postcard::to_allocvec(&inner)?;
        let sealed = keystone::SealedBox::seal(&owner.public.dh_pub, &bytes);
        self.post_envelope(&owner, keystone::Envelope::Response(sealed))
            .await
    }

    pub fn load_feed(&mut self) -> crate::Result<Vec<FeedPost>> {
        let posts = self.store()?.load_posts()?;
        let mut feed = Vec::new();
        for p in posts {
            let responses = self.store()?.load_responses_for(&p.id)?;

            let (reactions, comments): (Vec<_>, Vec<_>) = responses
                .into_iter()
                .partition(|r| r.kind == ResponseKind::Reaction);

            let content = PostContent {
                body: p.body,
                media: p.media,
            };

            feed.push(FeedPost {
                id: p.id,
                author: p.author,
                created_at: p.created_at,
                content,
                reactions: reactions
                    .into_iter()
                    .map(|r| FeedReaction {
                        author: r.author,
                        emoji: r.content,
                    })
                    .collect(),
                comments: comments
                    .into_iter()
                    .map(|c| FeedComment {
                        author: c.author,
                        text: c.content,
                    })
                    .collect(),
            });
        }
        Ok(feed)
    }

    pub async fn sync(&mut self) -> crate::Result<SyncResult> {
        let friends = self.store()?.load_friends()?;
        let mut all_sync_results = SyncResult::new();

        for friend in &friends {
            let sync_result = self.process_mailbox(friend).await?;
            all_sync_results.merge(sync_result);
        }

        Ok(all_sync_results)
    }
}

pub fn load_or_create_identity(storage: &Store) -> crate::Result<keystone::Identity> {
    let mut store = storage
        .lock()
        .map_err(|e| ClientError::PoisonError(e.to_string()))?;
    match store.load_identity()? {
        Some(id) => Ok(id),
        None => {
            let id = keystone::Identity::generate();
            store.save_identity(&id)?;
            Ok(id)
        }
    }
}
