use keystone::{
    identity::SigningPublicKey,
    post::{PostContent, PostId},
};

use crate::types::{stored_post::StoredPost, stored_response::StoredResponse};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Query error: {0}")]
    QueryError(String),
    #[error("Invalid response kind: {0} valid values are 0 (reaction) and 1 (comment)")]
    InvalidResponseKind(u8),
}

pub type StorageResult<T> = Result<T, StorageError>;

pub trait Storage: Send + Sync {
    /// Save our own identity to storage, only one identity can ever be stored.
    fn save_identity(&mut self, id: &keystone::Identity) -> StorageResult<()>;

    /// Load our own identity from storage, returns None if no identity is saved.
    fn load_identity(&mut self) -> StorageResult<Option<keystone::Identity>>;

    /// Save a friend to storage.
    fn save_friend(&mut self, f: &keystone::Friend) -> StorageResult<()>;

    /// Load all friends from storage.
    fn load_friends(&mut self) -> StorageResult<Vec<keystone::Friend>>;

    /// Load a specific friend by their public signing key, returns None if no friend is found.
    fn load_friend_by_sign_pub(
        &mut self,
        friend: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Friend>>;

    /// Save a profile to storage.
    fn save_profile(&mut self, p: &keystone::Profile) -> StorageResult<()>;

    /// Load a profile from storage by the owner's public signing key, returns None if no profile is found.
    fn load_profile(
        &mut self,
        owner: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Profile>>;

    /// Save a post to storage, returns true if the post was "new" and had not been saved before.
    fn save_post(
        &mut self,
        encrypted: &keystone::EncryptedPost,
        post: &PostContent,
    ) -> StorageResult<bool>;

    /// Load all posts from storage, returns an empty vector if no posts are found.
    fn load_posts(&mut self) -> StorageResult<Vec<StoredPost>>;

    /// Save a response to storage, returns true if the response was "new" and had not been saved before.
    fn save_response(&mut self, post_id: &PostId, response: &StoredResponse)
    -> StorageResult<bool>;

    /// Load all responses for a specific post from storage, returns an empty vector if no responses are found.
    fn load_responses_for(&mut self, post_id: &PostId) -> StorageResult<Vec<StoredResponse>>;

    /// Get the last index of a post for a specific friend, direction and epoch. Returns the last index.
    fn get_cursor(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize>;

    /// Set the last index of a post for a specific friend, direction and epoch.
    fn set_cursor(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> StorageResult<()>;
}
