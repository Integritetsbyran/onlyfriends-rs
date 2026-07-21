use keystone::ResponseBody::{Comment, Reaction};
use storage_common::storage::Storage;

use crate::{ClientError, RelayClient, epoch_now, mailbox_address, my_direction};

pub struct Account<Store: Storage> {
    pub storage: Store,
    pub identity: keystone::Identity,
    pub relay: RelayClient,
}

#[derive(Debug)]
pub struct SyncResult {
    pub new_posts: Vec<String>,
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
    pub id: [u8; 16],
    pub author: [u8; 32],
    pub created_at: u64,
    pub body: String, // already decrypted
    pub reactions: Vec<FeedReaction>,
    pub comments: Vec<FeedComment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedReaction {
    pub author: [u8; 32],
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedComment {
    pub author: [u8; 32],
    pub text: String, // already decrypted
}

impl<Store: Storage> Account<Store> {
    pub fn open(storage: Store, relay_url: &str) -> crate::Result<Self> {
        let identity = load_or_create_identity(&storage)?;
        let relay = RelayClient::new(relay_url);
        Ok(Account {
            storage,
            identity,
            relay,
        })
    }
    pub async fn add_friend(
        &self,
        their: &keystone::PublicIdentity,
        nickname: &str,
    ) -> crate::Result<keystone::Friend> {
        let friend = keystone::friend::add_friend(&self.identity, their, nickname);
        self.storage
            .save_friend(&friend)
            .map_err(crate::ClientError::from_storage)?;
        self.send_my_profile_to(&friend).await?;
        Ok(friend)
    }

    pub async fn send_post(&self, body: &str) -> crate::Result<[u8; 16]> {
        let friends = self
            .storage
            .load_friends()
            .map_err(ClientError::from_storage)?;
        let recipients: Vec<_> = friends.iter().map(|f| f.public.clone()).collect();

        // Always include self as a recipient so `create_post` always returns at
        // least one SealedPost.  That gives us the Post metadata (id, timestamp,
        // signature) needed to persist the post locally regardless of whether the
        // user has any friends yet.
        let mut recipients_with_self = recipients.clone();
        recipients_with_self.push(self.identity.public());
        let posts = keystone::post::create_post(&self.identity, body, &recipients_with_self);

        let post_id = posts.first().map(|p| p.post.id).unwrap_or([0u8; 16]);
        if let Some(first) = posts.first() {
            self.storage
                .save_post(&first.post, body)
                .map_err(ClientError::from_storage)?;
        }

        // Send only to actual friends; the trailing self-sealed post is unused.
        for (friend, post) in friends.iter().zip(posts.iter()) {
            let envelope: keystone::Envelope = keystone::Envelope::Post(post.clone());
            let bytes = postcard::to_allocvec(&envelope)?;
            let epoch = epoch_now(60 * 60 * 24);
            let addr = mailbox_address(
                &friend.pairwise_root,
                my_direction(&self.identity.public(), &friend.public),
                epoch,
            );

            self.relay.post_item(&addr, &bytes).await?;
        }
        Ok(post_id)
    }

    pub async fn process_mailbox(&self, friend: &keystone::Friend) -> crate::Result<SyncResult> {
        let direction = my_direction(&friend.public, &self.identity.public());
        let current = epoch_now(60 * 60 * 24);
        let start = current.saturating_sub(7);

        let mut sync_results = SyncResult::new();

        for e in start..=current {
            let addr = mailbox_address(&friend.pairwise_root, direction, e);
            let after = self
                .storage
                .get_cursor(&friend.public.sign_pub, direction, e)
                .map_err(ClientError::from_storage)?;
            let items = self.relay.get_items(&addr, after).await?;

            for item in &items {
                let Ok(envelope) = postcard::from_bytes::<keystone::Envelope>(item) else {
                    continue;
                };

                match envelope {
                    keystone::Envelope::Post(sealed_post) => {
                        let Ok(text) =
                            keystone::post::open_post(&self.identity, &friend.public, &sealed_post)
                        else {
                            continue;
                        };
                        if self
                            .storage
                            .save_post(&sealed_post.post, &text)
                            .map_err(ClientError::from_storage)?
                        {
                            sync_results.new_posts.push(text);
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

                        let existing = self
                            .storage
                            .load_profile(&profile.owner)
                            .map_err(ClientError::from_storage)?;
                        let is_newer = existing.is_none_or(|old| profile.version > old.version);
                        if is_newer {
                            self.storage
                                .save_profile(&profile)
                                .map_err(ClientError::from_storage)?;
                            sync_results.updated_profiles.push(profile);
                        }
                    }
                    keystone::Envelope::Response(sealed_box) => {
                        let Ok(rb) =
                            keystone::response::open_and_vouch(&self.identity, &sealed_box)
                        else {
                            continue;
                        };
                        for f in self
                            .storage
                            .load_friends()
                            .map_err(ClientError::from_storage)?
                        {
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
                self.storage
                    .set_cursor(&friend.public.sign_pub, direction, e, after + items.len())
                    .map_err(ClientError::from_storage)?;
            }
        }

        Ok(sync_results)
    }

    fn save_response(&self, rb: keystone::response::ResponseRebroadcast) -> crate::Result<()> {
        match rb.inner.body {
            Reaction { emoji } => {
                self.storage
                    .save_response(&rb.inner.post_id, &rb.inner.author, 0, emoji.as_str())
            }
            Comment { text } => {
                self.storage
                    .save_response(&rb.inner.post_id, &rb.inner.author, 1, text.as_str())
            }
        }
        .map_err(ClientError::from_storage)?;
        Ok(())
    }

    /// Set/update my own profile and push it to every friend.
    pub async fn set_profile(&self, display_name: &str, bio: &str) -> crate::Result<()> {
        let old_profile = self
            .storage
            .load_profile(&self.identity.public().sign_pub)
            .map_err(ClientError::from_storage)?;

        let new_profile = keystone::profile::create_profile(
            &self.identity,
            display_name,
            bio,
            old_profile.map_or(0, |p| p.version + 1),
        );
        self.storage
            .save_profile(&new_profile)
            .map_err(ClientError::from_storage)?;

        let friends = self
            .storage
            .load_friends()
            .map_err(ClientError::from_storage)?;
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
        let bytes = postcard::to_allocvec(&envelope)?;
        let addr = mailbox_address(
            &friend.pairwise_root,
            my_direction(&self.identity.public(), &friend.public),
            epoch_now(60 * 60 * 24),
        );
        self.relay.post_item(&addr, &bytes).await?;
        Ok(())
    }

    async fn send_my_profile_to(&self, friend: &keystone::Friend) -> crate::Result<()> {
        let Some(profile) = self
            .storage
            .load_profile(&self.identity.public().sign_pub)
            .map_err(ClientError::from_storage)?
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
        post_id: [u8; 16],
        post_author: &[u8; 32],
        emoji: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self
            .storage
            .load_friend_by_sign_pub(post_author)
            .map_err(ClientError::from_storage)?
        else {
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
        post_id: [u8; 16],
        post_author: &[u8; 32],
        text: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self
            .storage
            .load_friend_by_sign_pub(post_author)
            .map_err(ClientError::from_storage)?
        else {
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

    pub fn load_feed(&self) -> crate::Result<Vec<FeedPost>> {
        let posts = self
            .storage
            .load_posts()
            .map_err(ClientError::from_storage)?;
        let mut feed = Vec::new();
        for p in posts {
            let responses = self
                .storage
                .load_responses_for(&p.id)
                .map_err(ClientError::from_storage)?;
            let (reactions, comments): (Vec<_>, Vec<_>) =
                responses.into_iter().partition(|r| r.kind == 0);
            feed.push(FeedPost {
                id: p.id,
                author: p.author,
                created_at: p.created_at,
                body: p.body,
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

    pub async fn sync(&self) -> crate::Result<SyncResult> {
        let friends = self
            .storage
            .load_friends()
            .map_err(ClientError::from_storage)?;
        let mut all_sync_results = SyncResult::new();

        for friend in &friends {
            let sync_result = self.process_mailbox(friend).await?;
            all_sync_results.merge(sync_result);
        }

        Ok(all_sync_results)
    }
}

pub fn load_or_create_identity<Store: Storage>(
    storage: &Store,
) -> crate::Result<keystone::Identity> {
    match storage.load_identity().map_err(ClientError::from_storage)? {
        Some(id) => Ok(id),
        None => {
            let id = keystone::Identity::generate();
            storage
                .save_identity(&id)
                .map_err(ClientError::from_storage)?;
            Ok(id)
        }
    }
}
