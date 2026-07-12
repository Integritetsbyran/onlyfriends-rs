use crate::{RelayClient, Storage, epoch_now, mailbox_address, my_direction};

pub struct Account {
    pub storage: Storage,
    pub identity: keystone::Identity,
    pub relay: RelayClient,
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
    pub fn add_friend(
        &self,
        their: &keystone::PublicIdentity,
        nickname: &str,
    ) -> crate::Result<keystone::Friend> {
        let friend = keystone::friend::add_friend(&self.identity, their, nickname);
        self.storage.save_friend(&friend)?;
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

    pub async fn fetch_posts(
        &self,
        author_pub: &keystone::PublicIdentity,
        friend: &keystone::Friend,
        lookback_epochs: u64,
    ) -> crate::Result<Vec<String>> {
        let direction = my_direction(author_pub, &self.identity.public());
        let current = epoch_now(60 * 60 * 24);
        let start = current.saturating_sub(lookback_epochs);

        let mut results = Vec::new();

        for e in start..=current {
            let addr = mailbox_address(&friend.pairwise_root, direction, e);
            let after = self
                .storage
                .get_cursor(&friend.public.sign_pub, direction, e);
            let items = self.relay.get_items(&addr, after).await?;

            for item in items {
                let Ok(envelope) = postcard::from_bytes::<keystone::Envelope>(&item) else {
                    continue;
                };

                match envelope {
                    keystone::Envelope::Post(sealed_post) => {
                        let Ok(text) =
                            keystone::post::open_post(&self.identity, author_pub, &sealed_post)
                        else {
                            continue;
                        };
                        results.push(text);
                    }
                    _ => continue,
                }
            }
        }

        Ok(results)
    }

    pub async fn sync(&self) -> crate::Result<Vec<String>> {
        let friends = self.storage.load_friends()?;
        let mut all_new_posts = Vec::new();

        for friend in &friends {
            let new_posts = self.fetch_posts(&friend.public, friend, 7).await?;
            all_new_posts.extend(new_posts);
        }

        Ok(all_new_posts)
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
