pub mod account;
pub mod mailbox;
pub mod relay_client;

pub use account::{Account, FeedComment, FeedPost, FeedReaction};
pub use mailbox::{epoch_now, mailbox_address, my_direction};
pub use relay_client::RelayClient;

// Re-export the keystone types that the UI layer needs directly.
pub use keystone::{Friend, Profile, PublicIdentity};
use storage_common::storage::StorageError;

use crate::relay_client::RelayClientError;

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    #[error("Postcard error: {0}")]
    PostcardError(#[from] postcard::Error),
    #[error("Relay client error: {0}")]
    RelayClientError(#[from] RelayClientError),
    #[error("Trying to interact with someone who isn't my friend: {0}")]
    NotFriendError(&'static str),
    #[error("Failed to lock store: {0}")]
    PoisonError(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
