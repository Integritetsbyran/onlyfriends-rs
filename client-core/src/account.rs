use crate::{RelayClient, Storage, epoch_now, mailbox_address, my_direction};

pub struct Account {
    pub storage: Storage,
    pub identity: keystone::Identity,
    pub relay: RelayClient,
}

#[derive(Debug)]
pub struct SyncResult {
    pub new_posts: Vec<String>,
    pub updated_profiles: Vec<keystone::Profile>,
}

impl Account {
    pub fn open(db_path: &str, relay_url: &str) -> crate::Result<Account> {
        let storage = Storage::open(db_path)?;
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
        self.storage.save_friend(&friend)?;
        self.send_my_profile_to(&friend).await?;
        Ok(friend)
    }

    pub async fn send_post(&self, body: &str) -> crate::Result<()> {
        let friends = self.storage.load_friends()?;
        let recipients: Vec<_> = friends.iter().map(|f| f.public.clone()).collect();
        let posts = keystone::post::create_post(&self.identity, body, &recipients);

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
        Ok(())
    }

    pub async fn process_mailbox(&self, friend: &keystone::Friend) -> crate::Result<SyncResult> {
        let direction = my_direction(&friend.public, &self.identity.public());
        let current = epoch_now(60 * 60 * 24);
        let start = current.saturating_sub(7);

        let mut new_posts = Vec::new();
        let mut updated_profiles = Vec::new();

        for e in start..=current {
            let addr = mailbox_address(&friend.pairwise_root, direction, e);
            let after = self
                .storage
                .get_cursor(&friend.public.sign_pub, direction, e);
            let items = self.relay.get_items(&addr, after).await?;

            for item in &items {
                let Ok(envelope) = postcard::from_bytes::<keystone::Envelope>(&item) else {
                    continue;
                };

                match envelope {
                    keystone::Envelope::Post(sealed_post) => {
                        let Ok(text) =
                            keystone::post::open_post(&self.identity, &friend.public, &sealed_post)
                        else {
                            continue;
                        };
                        new_posts.push(text);
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

                        let existing = self.storage.load_profile(&profile.owner)?;
                        let is_newer = existing.map_or(true, |old| profile.version > old.version);
                        if is_newer {
                            self.storage.save_profile(&profile)?;
                            updated_profiles.push(profile);
                        }
                    }
                    keystone::Envelope::Response(_sealed_box) => todo!(),
                    keystone::Envelope::Rebroadcast(_sealed_box) => todo!(),
                }
            }
            if !items.is_empty() {
                self.storage
                    .set_cursor(&friend.public.sign_pub, direction, e, after + items.len())?;
            }
        }

        Ok(SyncResult {
            new_posts,
            updated_profiles,
        })
    }

    /// Set/update my own profile and push it to every friend.
    pub async fn set_profile(&self, display_name: &str, bio: &str) -> crate::Result<()> {
        let old_profile = self
            .storage
            .load_profile(&self.identity.public().sign_pub)?;

        let new_profile = keystone::profile::create_profile(
            &self.identity,
            display_name,
            bio,
            old_profile.map_or(0, |p| p.version + 1),
        );
        self.storage.save_profile(&new_profile)?;

        let friends = self.storage.load_friends()?;
        for friend in friends.iter() {
            self.send_my_profile_to(friend).await?
        }

        Ok(())
    }

    async fn send_my_profile_to(&self, friend: &keystone::Friend) -> crate::Result<()> {
        let Some(profile) = self
            .storage
            .load_profile(&self.identity.public().sign_pub)?
        else {
            return Ok(());
        };
        let profile_bytes = postcard::to_allocvec(&profile)?;
        let envelope = keystone::Envelope::Profile(keystone::SealedBox::seal(
            &friend.public.dh_pub,
            &profile_bytes,
        ));

        let bytes = postcard::to_allocvec(&envelope)?;

        let addr = mailbox_address(
            &friend.pairwise_root,
            my_direction(&self.identity.public(), &friend.public),
            epoch_now(60 * 60 * 24),
        );
        self.relay.post_item(&addr, &bytes).await?;
        Ok(())
    }

    pub async fn sync(&self) -> crate::Result<SyncResult> {
        let friends = self.storage.load_friends()?;
        let mut all_new_posts = Vec::new();
        let mut all_updated_profiles = Vec::new();

        for friend in &friends {
            let sync_result = self.process_mailbox(friend).await?;
            all_new_posts.extend(sync_result.new_posts);
            all_updated_profiles.extend(sync_result.updated_profiles);
        }

        Ok(SyncResult {
            new_posts: all_new_posts,
            updated_profiles: all_updated_profiles,
        })
    }
}

pub fn load_or_create_identity(storage: &Storage) -> crate::Result<keystone::Identity> {
    match storage.load_identity() {
        Some(id) => Ok(id),
        None => {
            let id = keystone::Identity::generate();
            storage.save_identity(&id)?;
            Ok(id)
        }
    }
}
