use crate::types::{stored_post::StoredPost, stored_response::StoredResponse};

pub trait Storable<Id> {
    fn id(&self) -> Id;
}

pub trait StorageError: std::error::Error + Send + Sync + 'static + std::fmt::Display {}

// Box<dyn Error> satisfies StorageError so it can serve as Storage::Error.
impl StorageError for Box<dyn std::error::Error + Send + Sync + 'static> {}

/// Object-safe mirror of [`Storage`] that erases the associated error type.
/// All methods return a boxed error so the trait can be used as `dyn DynStorage`.
pub trait DynStorage: Send + Sync + 'static {
    fn save_identity(&self, id: &keystone::Identity) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_identity(&self) -> Result<Option<keystone::Identity>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn save_friend(&self, f: &keystone::Friend) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_friends(&self) -> Result<Vec<keystone::Friend>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_friend_by_sign_pub(&self, sign_pub: &[u8; 32]) -> Result<Option<keystone::Friend>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn save_profile(&self, p: &keystone::Profile) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_profile(&self, owner_sign_pub: &[u8; 32]) -> Result<Option<keystone::Profile>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn save_post(&self, post: &keystone::Post, body: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_posts(&self) -> Result<Vec<crate::types::stored_post::StoredPost>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn save_response(&self, post_id: &[u8; 16], author: &[u8; 32], kind: u8, content: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn load_responses_for(&self, post_id: &[u8; 16]) -> Result<Vec<crate::types::stored_response::StoredResponse>, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn get_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn set_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64, last_index: usize) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// Blanket: any concrete `Storage` impl automatically becomes a `DynStorage`.
impl<S: Storage + Send + Sync + 'static> DynStorage for S {
    fn save_identity(&self, id: &keystone::Identity) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::save_identity(self, id).map_err(|e| Box::new(e) as _)
    }
    fn load_identity(&self) -> Result<Option<keystone::Identity>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_identity(self).map_err(|e| Box::new(e) as _)
    }
    fn save_friend(&self, f: &keystone::Friend) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::save_friend(self, f).map_err(|e| Box::new(e) as _)
    }
    fn load_friends(&self) -> Result<Vec<keystone::Friend>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_friends(self).map_err(|e| Box::new(e) as _)
    }
    fn load_friend_by_sign_pub(&self, sign_pub: &[u8; 32]) -> Result<Option<keystone::Friend>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_friend_by_sign_pub(self, sign_pub).map_err(|e| Box::new(e) as _)
    }
    fn save_profile(&self, p: &keystone::Profile) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::save_profile(self, p).map_err(|e| Box::new(e) as _)
    }
    fn load_profile(&self, owner_sign_pub: &[u8; 32]) -> Result<Option<keystone::Profile>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_profile(self, owner_sign_pub).map_err(|e| Box::new(e) as _)
    }
    fn save_post(&self, post: &keystone::Post, body: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::save_post(self, post, body).map_err(|e| Box::new(e) as _)
    }
    fn load_posts(&self) -> Result<Vec<crate::types::stored_post::StoredPost>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_posts(self).map_err(|e| Box::new(e) as _)
    }
    fn save_response(&self, post_id: &[u8; 16], author: &[u8; 32], kind: u8, content: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::save_response(self, post_id, author, kind, content).map_err(|e| Box::new(e) as _)
    }
    fn load_responses_for(&self, post_id: &[u8; 16]) -> Result<Vec<crate::types::stored_response::StoredResponse>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::load_responses_for(self, post_id).map_err(|e| Box::new(e) as _)
    }
    fn get_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::get_cursor(self, friend_sign_pub, direction, epoch).map_err(|e| Box::new(e) as _)
    }
    fn set_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64, last_index: usize) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Storage::set_cursor(self, friend_sign_pub, direction, epoch, last_index).map_err(|e| Box::new(e) as _)
    }
}

/// Bridge: `Box<dyn DynStorage>` itself implements `Storage`, so `Account<Box<dyn DynStorage>>`
/// compiles without any generics leaking into the UI.
impl Storage for Box<dyn DynStorage> {
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn save_identity(&self, id: &keystone::Identity) -> Result<usize, Self::Error> { (**self).save_identity(id) }
    fn load_identity(&self) -> Result<Option<keystone::Identity>, Self::Error> { (**self).load_identity() }
    fn save_friend(&self, f: &keystone::Friend) -> Result<usize, Self::Error> { (**self).save_friend(f) }
    fn load_friends(&self) -> Result<Vec<keystone::Friend>, Self::Error> { (**self).load_friends() }
    fn load_friend_by_sign_pub(&self, sign_pub: &[u8; 32]) -> Result<Option<keystone::Friend>, Self::Error> { (**self).load_friend_by_sign_pub(sign_pub) }
    fn save_profile(&self, p: &keystone::Profile) -> Result<usize, Self::Error> { (**self).save_profile(p) }
    fn load_profile(&self, owner_sign_pub: &[u8; 32]) -> Result<Option<keystone::Profile>, Self::Error> { (**self).load_profile(owner_sign_pub) }
    fn save_post(&self, post: &keystone::Post, body: &str) -> Result<bool, Self::Error> { (**self).save_post(post, body) }
    fn load_posts(&self) -> Result<Vec<crate::types::stored_post::StoredPost>, Self::Error> { (**self).load_posts() }
    fn save_response(&self, post_id: &[u8; 16], author: &[u8; 32], kind: u8, content: &str) -> Result<bool, Self::Error> { (**self).save_response(post_id, author, kind, content) }
    fn load_responses_for(&self, post_id: &[u8; 16]) -> Result<Vec<crate::types::stored_response::StoredResponse>, Self::Error> { (**self).load_responses_for(post_id) }
    fn get_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64) -> Result<usize, Self::Error> { (**self).get_cursor(friend_sign_pub, direction, epoch) }
    fn set_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64, last_index: usize) -> Result<usize, Self::Error> { (**self).set_cursor(friend_sign_pub, direction, epoch, last_index) }
}

pub trait Storage {
    type Error: StorageError;

    fn save_identity(&self, id: &keystone::Identity) -> Result<usize, Self::Error>;
    fn load_identity(&self) -> Result<Option<keystone::Identity>, Self::Error>;
    fn save_friend(&self, f: &keystone::Friend) -> Result<usize, Self::Error>;
    fn load_friends(&self) -> Result<Vec<keystone::Friend>, Self::Error>;
    fn load_friend_by_sign_pub(
        &self,
        sign_pub: &[u8; 32],
    ) -> Result<Option<keystone::Friend>, Self::Error>;
    fn save_profile(&self, p: &keystone::Profile) -> Result<usize, Self::Error>;
    fn load_profile(
        &self,
        owner_sign_pub: &[u8; 32],
    ) -> Result<Option<keystone::Profile>, Self::Error>;
    fn save_post(&self, post: &keystone::Post, body: &str) -> Result<bool, Self::Error>;
    fn load_posts(&self) -> Result<Vec<StoredPost>, Self::Error>;
    fn save_response(
        &self,
        post_id: &[u8; 16],
        author: &[u8; 32],
        kind: u8,
        content: &str,
    ) -> Result<bool, Self::Error>;
    fn load_responses_for(&self, post_id: &[u8; 16]) -> Result<Vec<StoredResponse>, Self::Error>;
    fn get_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
    ) -> Result<usize, Self::Error>;
    fn set_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
        last_index: usize,
    ) -> Result<usize, Self::Error>;
}
