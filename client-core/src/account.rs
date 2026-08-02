use keystone::Envelope;
use keystone::envelope::LetterId;
use keystone::identity::SigningPublicKey;
use keystone::message::Message;
use keystone::post::PostContent;
use onlyfriends_time::days_since_epoch;
use storage_common::storage::Storage;
use storage_common::types::{relay_config::RelayConfig, stored_response::ResponseKind};
use tokio::sync::{Mutex, MutexGuard};

use std::sync::Arc;

use crate::{ClientError, RelayClient, mailbox_address, my_direction};

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
    pub id: LetterId,
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
    #[inline(always)]
    pub async fn store(&self) -> MutexGuard<'_, dyn Storage + 'static> {
        self.storage.lock().await
    }

    pub async fn open(storage: Store) -> crate::Result<Option<Self>> {
        let identity = storage.lock().await.load_identity().await?;
        let relay_config = storage.lock().await.load_relay_config().await?;

        let Some((identity, RelayConfig { url })) = identity.zip(relay_config) else {
            return Ok(None);
        };

        let relay = RelayClient::new(url);
        Ok(Some(Account {
            storage,
            identity,
            relay,
        }))
    }

    pub async fn create_new(storage: Store, relay_url: &str) -> crate::Result<Self> {
        let identity = load_or_create_identity(&storage).await?;
        storage
            .lock()
            .await
            .save_relay_config(&RelayConfig {
                url: relay_url.to_string(),
            })
            .await?;
        let relay = RelayClient::new(relay_url);
        Ok(Account {
            storage,
            identity,
            relay,
        })
    }

    pub async fn set_relay_url(&mut self, relay_url: String) -> crate::Result<()> {
        self.relay = RelayClient::new(&relay_url);
        self.store()
            .await
            .save_relay_config(&RelayConfig { url: relay_url })
            .await?;
        Ok(())
    }
    pub async fn add_friend(
        &mut self,
        their: &keystone::PublicIdentity,
        nickname: &str,
    ) -> crate::Result<keystone::Friend> {
        let friend = keystone::friend::add_friend(&self.identity, their, nickname);
        self.store().await.save_friend(&friend).await?;
        self.send_my_profile_to(&friend).await?;
        Ok(friend)
    }

    /// Send `post` to all friends.
    ///
    /// Returns the letter id, or `None` if you have no friends.
    pub async fn send_text_post(
        &mut self,
        body: impl Into<String>,
    ) -> crate::Result<Option<LetterId>> {
        self.send_post(&PostContent::from_body(body)).await
    }

    /// Send `post` to all friends.
    ///
    /// Returns the letter id, or `None` if you have no friends.
    pub async fn send_post(&mut self, post: &PostContent) -> crate::Result<Option<LetterId>> {
        let friends = self.store().await.load_friends().await?;
        let recipients: Vec<_> = friends.iter().map(|f| f.public.clone()).collect();

        // Always include self as a recipient so `create_post` always returns at
        // least one SealedPost.  That gives us the Post metadata (id, timestamp,
        // signature) needed to persist the post locally regardless of whether the
        // user has any friends yet.
        let mut recipients_with_self = recipients.clone();
        recipients_with_self.push(self.identity.public());
        let envelopes = keystone::Envelope::seal_envelope(
            &self.identity,
            &Message::Post(post.clone()),
            &recipients_with_self,
        );

        let post_id = envelopes.first().map(|p| p.letter.id);
        if let Some(first) = envelopes.first() {
            self.store().await.save_post(&first.letter, post).await?;
        }

        // Send only to actual friends; the trailing self-sealed post is unused.
        for (friend, envelope) in friends.iter().zip(envelopes.iter()) {
            let epoch = days_since_epoch();
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
        let current = days_since_epoch();
        let start = current.saturating_sub(7);

        let mut sync_results = SyncResult::new();

        for e in start..=current {
            let addr = mailbox_address(&friend.pairwise_root, direction, e);
            let after = self
                .store()
                .await
                .get_cursor(&friend.public.sign_pub, direction, e)
                .await?;
            let items = self.relay.get_items(&addr, after).await?;

            for item in &items {
                let Ok(envelope) = postcard::from_bytes::<keystone::Envelope>(item) else {
                    continue;
                };

                let Ok(letter) = envelope.open_envelope(&self.identity, &friend.public) else {
                    continue;
                };

                match letter {
                    Message::Post(post) => {
                        if self
                            .store()
                            .await
                            .save_post(&envelope.letter, &post)
                            .await?
                        {
                            sync_results.new_posts.push(post);
                        }
                    }
                    Message::Profile(profile) => {
                        if keystone::profile::verify_profile(&profile).is_err() {
                            continue;
                        }

                        let existing = self.store().await.load_profile(&profile.owner).await?;
                        let is_newer = existing.is_none_or(|old| profile.version > old.version);
                        if is_newer {
                            self.store().await.save_profile(&profile).await?;
                            sync_results.updated_profiles.push(profile);
                        }
                    }
                    Message::Response(response) => {
                        // Vouch for the response by signing it
                        use keystone::Signable;
                        let vouch_sig = response.sign_with(&self.identity.signing_key());
                        let rb = keystone::response::ResponseRebroadcast {
                            inner: response,
                            vouch_sig,
                        };

                        let friends = self.store().await.load_friends().await?;
                        for f in friends {
                            let envelopes = Envelope::seal_envelope(
                                &self.identity,
                                &Message::Rebroadcast(rb.clone()),
                                std::slice::from_ref(&f.public),
                            );
                            if let Some(envelope) = envelopes.into_iter().next() {
                                self.post_envelope(&f, envelope).await?;
                            }
                        }
                        sync_results.new_responses.push(rb.inner.clone());
                        self.save_response(rb).await?;
                    }
                    Message::Rebroadcast(rb) => {
                        sync_results.new_responses.push(rb.inner.clone());
                        self.save_response(rb).await?;
                    }
                }
            }
            if !items.is_empty() {
                self.store()
                    .await
                    .set_cursor(friend.public.sign_pub, direction, e, after + items.len())
                    .await?;
            }
        }

        Ok(sync_results)
    }

    async fn save_response(
        &self,
        rb: keystone::response::ResponseRebroadcast,
    ) -> crate::Result<()> {
        let letter_id = rb.inner.letter_id;
        self.store()
            .await
            .save_response(&letter_id, &rb.inner.into())
            .await?;
        Ok(())
    }

    /// Set/update my own profile and push it to every friend.
    pub async fn set_profile(&self, display_name: &str, bio: &str) -> crate::Result<()> {
        let old_profile = self
            .store()
            .await
            .load_profile(&self.identity.public().sign_pub)
            .await?;

        let new_profile = keystone::profile::create_profile(
            &self.identity,
            display_name,
            bio,
            old_profile.map_or(0, |p| p.version + 1),
        );
        self.store().await.save_profile(&new_profile).await?;

        let friends = self.store().await.load_friends().await?;
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
            days_since_epoch(),
        );
        self.relay.post_item(&addr, &envelope).await?;
        Ok(())
    }

    async fn send_my_profile_to(&self, friend: &keystone::Friend) -> crate::Result<()> {
        let Some(profile) = self
            .store()
            .await
            .load_profile(&self.identity.public().sign_pub)
            .await?
        else {
            return Ok(());
        };
        let envelopes = Envelope::seal_envelope(
            &self.identity,
            &Message::Profile(profile),
            std::slice::from_ref(&friend.public),
        );
        let envelope = envelopes.into_iter().next().expect("one recipient");

        self.post_envelope(friend, envelope).await?;
        Ok(())
    }

    pub async fn react(
        &self,
        letter_id: LetterId,
        post_author: &SigningPublicKey,
        emoji: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self
            .store()
            .await
            .load_friend_by_sign_pub(post_author)
            .await?
        else {
            return Err(ClientError::NotFriendError("react to a post"));
        };
        let inner = keystone::response::create_response(
            &self.identity,
            letter_id,
            keystone::ResponseBody::Reaction {
                emoji: emoji.to_string(),
            },
        );

        let envelopes = Envelope::seal_envelope(
            &self.identity,
            &Message::Response(inner),
            std::slice::from_ref(&owner.public),
        );
        let envelope = envelopes.into_iter().next().expect("one recipient");
        self.post_envelope(&owner, envelope).await
    }

    pub async fn comment(
        &self,
        letter_id: LetterId,
        post_author: &SigningPublicKey,
        text: &str,
    ) -> crate::Result<()> {
        let Some(owner) = self
            .store()
            .await
            .load_friend_by_sign_pub(post_author)
            .await?
        else {
            return Err(ClientError::NotFriendError("comment on a post"));
        };
        let inner = keystone::response::create_response(
            &self.identity,
            letter_id,
            keystone::ResponseBody::Comment {
                text: text.to_string(),
            },
        );

        let message = Message::Response(inner);
        let envelopes = Envelope::seal_envelope(
            &self.identity,
            &message,
            std::slice::from_ref(&owner.public),
        );
        let envelope = envelopes.into_iter().next().expect("one recipient");

        self.post_envelope(&owner, envelope).await
    }

    pub async fn load_feed(&mut self) -> crate::Result<Vec<FeedPost>> {
        let posts = self.store().await.load_posts().await?;
        let mut feed = Vec::new();
        for p in posts {
            let responses = self.store().await.load_responses_for(&p.id).await?;

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
        let friends = self.store().await.load_friends().await?;
        let mut all_sync_results = SyncResult::new();

        for friend in &friends {
            let sync_result = self.process_mailbox(friend).await?;
            all_sync_results.merge(sync_result);
        }

        Ok(all_sync_results)
    }
}

pub async fn load_or_create_identity(storage: &Store) -> crate::Result<keystone::Identity> {
    let mut store = storage.lock().await;
    match store.load_identity().await? {
        Some(id) => Ok(id),
        None => {
            let id = keystone::Identity::generate();
            store.save_identity(&id).await?;
            Ok(id)
        }
    }
}
