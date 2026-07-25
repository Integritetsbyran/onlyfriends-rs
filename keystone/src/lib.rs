pub mod crypto;
pub mod envelope;
pub mod friend;
pub mod identity;
pub mod labels;
pub mod media;
pub mod post;
pub mod profile;
pub mod response;
pub mod signing;
pub mod util;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Aead,      // AEAD open failed: wrong key or tampered ciphertext
    Signature, // signature didn't verify
    Serialize, // (de)serialization failed
    BadKey,    // malformed key/public value
}

impl From<postcard::Error> for crate::Error {
    fn from(_: postcard::Error) -> Self {
        crate::Error::Serialize
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub use crypto::SealedBox;
pub use envelope::Envelope;
pub use friend::Friend;
pub use identity::{Identity, PublicIdentity};
pub use post::{EncryptedPost, SealedPost};
pub use profile::Profile;
pub use response::ResponseBody;
