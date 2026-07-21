use crate::types::{stored_post::StoredPost, stored_response::StoredResponse};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Query error: {0}")]
    QueryError(String),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

pub trait Storage {
    fn save_identity(&self, id: &keystone::Identity) -> StorageResult<usize>;
    fn load_identity(&self) -> StorageResult<Option<keystone::Identity>>;
    fn save_friend(&self, f: &keystone::Friend) -> StorageResult<usize>;
    fn load_friends(&self) -> StorageResult<Vec<keystone::Friend>>;
    fn load_friend_by_sign_pub(
        &self,
        sign_pub: &[u8; 32],
    ) -> StorageResult<Option<keystone::Friend>>;
    fn save_profile(&self, p: &keystone::Profile) -> StorageResult<usize>;
    fn load_profile(&self, owner_sign_pub: &[u8; 32]) -> StorageResult<Option<keystone::Profile>>;
    fn save_post(&self, post: &keystone::Post, body: &str) -> StorageResult<bool>;
    fn load_posts(&self) -> StorageResult<Vec<StoredPost>>;
    fn save_response(
        &self,
        post_id: &[u8; 16],
        author: &[u8; 32],
        kind: u8,
        content: &str,
    ) -> StorageResult<bool>;
    fn load_responses_for(&self, post_id: &[u8; 16]) -> StorageResult<Vec<StoredResponse>>;
    fn get_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize>;
    fn set_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> StorageResult<()>;
}
