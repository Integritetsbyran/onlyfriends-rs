pub mod account;
pub mod mailbox;
pub mod relay_client;
pub mod storage;

pub use account::{Account, FeedComment, FeedPost, FeedReaction};
pub use mailbox::{epoch_now, mailbox_address, my_direction};
pub use relay_client::RelayClient;
pub use storage::Storage;

// Re-export the keystone types that the UI layer needs directly.
pub use keystone::{Friend, Profile, PublicIdentity};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
