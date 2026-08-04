use crate::media::Media;
use onlyfriends_time::seconds_since_epoch;
use serde::{Deserialize, Serialize};

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostContent {
    /// The plain-text content of a post.
    pub body: String,

    /// Unix epoch of when the post was created.
    pub created_at: u64,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
}

impl PostContent {
    pub fn from_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            created_at: seconds_since_epoch(),
            media: Default::default(),
        }
    }
}
