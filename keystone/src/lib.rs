pub mod crypto;
pub mod friend;
pub mod identity;
pub mod labels;
pub mod post;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Aead,      // AEAD open failed: wrong key or tampered ciphertext
    Signature, // signature didn't verify
    Serialize, // (de)serialization failed
    BadKey,    // malformed key/public value
}

pub type Result<T> = core::result::Result<T, Error>;

pub use crypto::SealedBox;
pub use friend::Friend;
pub use identity::{Identity, PublicIdentity};
pub use post::{Post, PostEnvelope};
