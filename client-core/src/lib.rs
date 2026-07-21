pub mod account;
pub mod mailbox;
pub mod relay_client;

pub use account::{Account, FeedComment, FeedPost, FeedReaction};
pub use mailbox::{epoch_now, mailbox_address, my_direction};
pub use relay_client::RelayClient;

// Re-export the keystone types that the UI layer needs directly.
pub use keystone::{Friend, Profile, PublicIdentity};

use crate::relay_client::RelayClientError;

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Postcard error: {0}")]
    PostcardError(#[from] postcard::Error),
    #[error("Relay client error: {0}")]
    RelayClientError(#[from] RelayClientError),
    #[error("Trying to interact with someone who isn't my friend: {0}")]
    NotFriendError(&'static str),
}

impl ClientError {
    // TODO: Would prefer to use thiserror to implement From<T: StorageError> for ClientError
    // however, that causes a conflict because (e.g.) PostcardError theoretically also can implement the trait
    // which would in turn cause a conflict.
    pub fn from_storage(e: impl storage_common::storage::StorageError) -> Self {
        ClientError::StorageError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;
