use crate::inner::tables::{
    cursor::WebCursor, friend::WebFriend, identity::WebIdentity, post::WebPost,
    profile::WebProfile, relay_config::WebRelayConfig, response::WebResponse,
};
use deli::{Database, KeyRange, Model};
use keystone::{envelope::LetterId, identity::SigningPublicKey};
use storage_common::{
    storage::{Storage, StorageError, StorageResult},
    types::{relay_config::RelayConfig, stored_post::StoredPost, stored_response::StoredResponse},
};

mod tables;

#[derive(Debug, thiserror::Error)]
pub enum WebStorageError {
    #[error("Deli error: {0}")]
    DeliError(#[from] deli::Error),
}

impl From<WebStorageError> for StorageError {
    fn from(value: WebStorageError) -> Self {
        match value {
            WebStorageError::DeliError(error) => Self::QueryError(format!("Deli error: {error:?}")),
        }
    }
}

pub struct WebStorage {
    db: Database,
}

// TODO: Consider making storage trait take owned models so that we don't need clone when saving them.
impl WebStorage {
    pub async fn open(db_name: &str) -> Result<Self, WebStorageError> {
        let db = Database::builder(db_name)
            .version(1)
            .add_model::<WebIdentity>()
            .add_model::<WebRelayConfig>()
            .add_model::<WebFriend>()
            .add_model::<WebProfile>()
            .add_model::<WebPost>()
            .add_model::<WebResponse>()
            .add_model::<WebCursor>()
            .build()
            .await?;

        Ok(WebStorage { db })
    }

    async fn save_identity_inner(&self, id: &keystone::Identity) -> Result<(), WebStorageError> {
        let web_identity = WebIdentity::from(id.master_seed);
        let tx = self
            .db
            .transaction()
            .with_model::<WebIdentity>()
            .writable()
            .build()?;
        WebIdentity::with_transaction(&tx)?
            .add(&web_identity)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn load_identity_inner(&mut self) -> Result<Option<keystone::Identity>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebIdentity>().build()?;
        let mut identities: Vec<WebIdentity> = WebIdentity::with_transaction(&tx)?
            .get_all(.., None)
            .await?;

        tx.commit().await?;

        Ok(identities.pop().map(|id| id.into()))
    }

    async fn save_relay_config_inner(
        &mut self,
        config: &RelayConfig,
    ) -> Result<(), WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebRelayConfig>()
            .writable()
            .build()?;
        let web_profile = WebRelayConfig::from(config.clone());
        WebRelayConfig::with_transaction(&tx)?
            .update(&web_profile)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn load_relay_config_inner(&mut self) -> Result<Option<RelayConfig>, WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebRelayConfig>()
            .build()?;
        let mut relay_configs: Vec<WebRelayConfig> = WebRelayConfig::with_transaction(&tx)?
            .get_all(.., None)
            .await?;

        tx.commit().await?;

        Ok(relay_configs.pop().map(|config| config.into()))
    }

    async fn save_friend_inner(&self, f: &keystone::Friend) -> Result<(), WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebFriend>()
            .writable()
            .build()?;
        let web_friend = WebFriend::from(f.clone());
        WebFriend::with_transaction(&tx)?.add(&web_friend).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn load_friends_inner(&mut self) -> Result<Vec<keystone::Friend>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebFriend>().build()?;
        let web_friends: Vec<WebFriend> =
            WebFriend::with_transaction(&tx)?.get_all(.., None).await?;
        tx.commit().await?;

        Ok(web_friends.into_iter().map(|f| f.into()).collect())
    }

    async fn load_friend_by_sign_pub_inner(
        &mut self,
        friend: &SigningPublicKey,
    ) -> Result<Option<keystone::Friend>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebFriend>().build()?;

        let web_friend: Option<WebFriend> = WebFriend::with_transaction(&tx)?.get(friend).await?;
        tx.commit().await?;

        Ok(web_friend.map(|f| f.into()))
    }

    async fn save_profile_inner(&mut self, p: &keystone::Profile) -> Result<(), WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebProfile>()
            .writable()
            .build()?;
        let web_profile = WebProfile::from(p.clone());
        WebProfile::with_transaction(&tx)?
            .update(&web_profile)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn load_profile_inner(
        &mut self,
        owner: &SigningPublicKey,
    ) -> Result<Option<keystone::Profile>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebProfile>().build()?;
        let web_profile: Option<WebProfile> = WebProfile::with_transaction(&tx)?.get(owner).await?;

        tx.commit().await?;
        Ok(web_profile.map(|p| p.into()))
    }

    async fn save_post_inner(
        &mut self,
        author: &SigningPublicKey,
        letter: &keystone::Letter,
        meta: &keystone::message::MessageMeta,
        post: &keystone::post::Post,
    ) -> Result<bool, WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebPost>()
            .writable()
            .build()?;
        let web_post = WebPost::new(author, letter.clone(), meta, post.clone());
        WebPost::with_transaction(&tx)?.add(&web_post).await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn load_posts_inner(&mut self) -> Result<Vec<StoredPost>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebPost>().build()?;
        let web_posts: Vec<WebPost> = WebPost::with_transaction(&tx)?.get_all(.., None).await?;
        tx.commit().await?;
        Ok(web_posts.into_iter().map(|p| p.into()).collect())
    }

    async fn save_response_inner(
        &mut self,
        letter_id: &LetterId,
        response: &StoredResponse,
    ) -> Result<bool, WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebResponse>()
            .writable()
            .build()?;
        let web_response = WebResponse::new(*letter_id, response.clone());
        WebResponse::with_transaction(&tx)?
            .add(&web_response)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn load_responses_for_inner(
        &mut self,
        letter_id: &LetterId,
    ) -> Result<Vec<StoredResponse>, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebResponse>().build()?;

        let web_responses: Vec<WebResponse> = WebResponse::with_transaction(&tx)?
            .by_letter_id()?
            .get_all(KeyRange::from(letter_id), None)
            .await?;
        tx.commit().await?;
        Ok(web_responses.into_iter().map(|r| r.into()).collect())
    }

    async fn get_cursor_inner(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> Result<usize, WebStorageError> {
        let tx = self.db.transaction().with_model::<WebCursor>().build()?;
        let web_cursor: Option<WebCursor> = WebCursor::with_transaction(&tx)?
            .get(&(*friend, direction, epoch))
            .await?;
        tx.commit().await?;

        Ok(web_cursor.map_or(0, |c| c.last_index))
    }

    async fn set_cursor_inner(
        &mut self,
        friend: SigningPublicKey,
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> Result<(), WebStorageError> {
        let tx = self
            .db
            .transaction()
            .with_model::<WebCursor>()
            .writable()
            .build()?;

        let mut web_cursor: Option<WebCursor> = WebCursor::with_transaction(&tx)?
            .get(&(friend, direction, epoch))
            .await?;

        if let Some(web_cursor) = web_cursor.as_mut() {
            web_cursor.last_index = last_index;
            WebCursor::with_transaction(&tx)?.update(web_cursor).await?;
        } else {
            let new_web_cursor = WebCursor::new(friend, direction, epoch, last_index);
            WebCursor::with_transaction(&tx)?
                .add(&new_web_cursor)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}

// SAFETY: WebAssembly is single-threaded; there is no concurrent access.
unsafe impl Send for WebStorage {}

#[async_trait::async_trait(?Send)]
impl Storage for WebStorage {
    async fn save_identity(&mut self, id: &keystone::Identity) -> StorageResult<()> {
        self.save_identity_inner(id).await?;
        Ok(())
    }

    async fn save_relay_config(&mut self, config: &RelayConfig) -> StorageResult<()> {
        self.save_relay_config_inner(config).await?;
        Ok(())
    }

    async fn load_relay_config(&mut self) -> StorageResult<Option<RelayConfig>> {
        Ok(self.load_relay_config_inner().await?)
    }

    async fn load_identity(&mut self) -> StorageResult<Option<keystone::Identity>> {
        Ok(self.load_identity_inner().await?)
    }

    async fn save_friend(&mut self, f: &keystone::Friend) -> StorageResult<()> {
        self.save_friend_inner(f).await?;
        Ok(())
    }

    async fn load_friends(&mut self) -> StorageResult<Vec<keystone::Friend>> {
        Ok(self.load_friends_inner().await?)
    }

    async fn load_friend_by_sign_pub(
        &mut self,
        friend: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Friend>> {
        Ok(self.load_friend_by_sign_pub_inner(friend).await?)
    }

    async fn save_profile(&mut self, p: &keystone::Profile) -> StorageResult<()> {
        self.save_profile_inner(p).await?;
        Ok(())
    }

    async fn load_profile(
        &mut self,
        owner: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Profile>> {
        Ok(self.load_profile_inner(owner).await?)
    }

    async fn save_post(
        &mut self,
        author: &SigningPublicKey,
        letter: &keystone::Letter,
        meta: &keystone::message::MessageMeta,
        post: &keystone::post::Post,
    ) -> StorageResult<bool> {
        Ok(self.save_post_inner(author, letter, meta, post).await?)
    }

    async fn load_posts(&mut self) -> StorageResult<Vec<StoredPost>> {
        Ok(self.load_posts_inner().await?)
    }

    async fn save_response(
        &mut self,
        letter_id: &LetterId,
        response: &StoredResponse,
    ) -> StorageResult<bool> {
        Ok(self.save_response_inner(letter_id, response).await?)
    }

    async fn load_responses_for(
        &mut self,
        letter_id: &LetterId,
    ) -> StorageResult<Vec<StoredResponse>> {
        Ok(self.load_responses_for_inner(letter_id).await?)
    }

    async fn get_cursor(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize> {
        Ok(self.get_cursor_inner(friend, direction, epoch).await?)
    }

    async fn set_cursor(
        &mut self,
        friend: SigningPublicKey,
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> StorageResult<()> {
        self.set_cursor_inner(friend, direction, epoch, last_index)
            .await?;
        Ok(())
    }
}
