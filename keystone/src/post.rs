use crate::crypto;
use crate::envelope::Envelope;
use crate::identity::{Identity, PublicIdentity};
use crate::media::Media;
use serde::{Deserialize, Serialize};

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Post {
    /// The plain-text content of a post.
    pub body: String,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostId(pub [u8; 16]);

impl PostId {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }
}

impl Post {
    pub fn from_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            media: Default::default(),
        }
    }
}

pub fn seal_post(author: &Identity, post: &Post, recipients: &[PublicIdentity]) -> Envelope {
    // TODO: avoid clone
    let message = crate::Message::Post(post.clone());
    Envelope::seal(author, &message, recipients)
}
