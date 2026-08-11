use duplicate::duplicate_item;
use onlyfriends_time::seconds_since_epoch;
use serde::{Deserialize, Serialize};

use crate::Profile;
use crate::post::Post;
use crate::response::{Response, ResponseRebroadcast};

/// An onlyfriends wire message.
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub content: MessageContent,
    pub meta: MessageMeta,
}

/// Metadata that is common to all [`MessageContent`] variants.
#[derive(Serialize, Deserialize)]
pub struct MessageMeta {
    pub created_at: u64,
}

/// All message types supported by the onlyfriends wire format.
///
/// # Compatibility
/// The postcard wire format is stable, and describes tagged unions (enums) with a `varint(u32)` discriminant.
/// To maintain cross-compatibility between versions, we explicitly set the discriminant for each variant.
/// A discriminant should never be re-used, therefore removing variants is discauraged.
#[derive(Serialize, Deserialize)]
#[repr(u32)] // postcard uses u32 to tag enums
pub enum MessageContent {
    Post(Post) = 0,
    Profile(Profile) = 1,
    Response(Response) = 2,
    ResponseRebroadcast(ResponseRebroadcast) = 3,
}

impl Message {
    /// Construct a new `Message` from a content.
    ///
    /// The message is stamped with the current time.
    pub fn new(content: impl Into<MessageContent>) -> Self {
        Self {
            content: content.into(),
            meta: MessageMeta {
                created_at: seconds_since_epoch(),
            },
        }
    }
}

#[duplicate_item(
    T;
    [Post];
    [Profile];
    [Response];
    [ResponseRebroadcast];
)]
impl From<T> for MessageContent {
    fn from(value: T) -> Self {
        MessageContent::T(value)
    }
}
