use keystone::{envelope::LetterId, identity::SigningPublicKey, message::MessageMeta, post::Post};

use crate::types::{
    relay_config::RelayConfig, stored_post::StoredPost, stored_response::StoredResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Query error: {0}")]
    QueryError(String),
    #[error("Invalid response kind: {0} valid values are 0 (reaction) and 1 (comment)")]
    InvalidResponseKind(u8),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Storage: Send {
    /// Save our own identity to storage, only one identity can ever be stored.
    async fn save_identity(&mut self, id: &keystone::Identity) -> StorageResult<()>;

    /// Load our own identity from storage, returns None if no identity is saved.
    async fn load_identity(&mut self) -> StorageResult<Option<keystone::Identity>>;

    /// Save the relay config to storage, only one can ever be stored.
    async fn save_relay_config(&mut self, config: &RelayConfig) -> StorageResult<()>;

    /// Load the relay config from storage, if any.
    async fn load_relay_config(&mut self) -> StorageResult<Option<RelayConfig>>;

    /// Save a friend to storage.
    async fn save_friend(&mut self, f: &keystone::Friend) -> StorageResult<()>;

    /// Load all friends from storage.
    async fn load_friends(&mut self) -> StorageResult<Vec<keystone::Friend>>;

    /// Load a specific friend by their public signing key, returns None if no friend is found.
    async fn load_friend_by_sign_pub(
        &mut self,
        friend: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Friend>>;

    /// Save a profile to storage.
    async fn save_profile(&mut self, p: &keystone::Profile) -> StorageResult<()>;

    /// Load a profile from storage by the owner's public signing key, returns None if no profile is found.
    async fn load_profile(
        &mut self,
        owner: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Profile>>;

    /// Save a post to storage, returns true if the post was "new" and had not been saved before.
    async fn save_post(
        &mut self,
        author: &SigningPublicKey,
        letter: &keystone::Letter,
        meta: &MessageMeta,
        post: &Post,
    ) -> StorageResult<bool>;

    /// Load all posts from storage, returns an empty vector if no posts are found.
    async fn load_posts(&mut self) -> StorageResult<Vec<StoredPost>>;

    /// Save a response to storage, returns true if the response was "new" and had not been saved before.
    async fn save_response(
        &mut self,
        letter_id: &LetterId,
        response: &StoredResponse,
    ) -> StorageResult<bool>;

    /// Load all responses for a specific letter from storage, returns an empty vector if no responses are found.
    async fn load_responses_for(
        &mut self,
        letter_id: &LetterId,
    ) -> StorageResult<Vec<StoredResponse>>;

    /// Get the last index of a post for a specific friend, direction and epoch. Returns the last index.
    async fn get_cursor(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize>;

    /// Set the last index of a post for a specific friend, direction and epoch.
    async fn set_cursor(
        &mut self,
        friend: SigningPublicKey,
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> StorageResult<()>;
}
