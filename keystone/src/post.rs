use crate::{PublicKeySealed, Signable};
use crate::crypto::{self};
use crate::identity::{Identity, PublicIdentity, SigningPublicKey};
use crate::media::Media;
use crate::signing::Signature;
use onlyfriends_time::seconds_since_epoch;
use serde::{Deserialize, Serialize};

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostContent {
    /// The plain-text content of a post.
    pub body: String,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
}

impl PostContent {
    pub fn from_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            media: Default::default(),
        }
    }
}

