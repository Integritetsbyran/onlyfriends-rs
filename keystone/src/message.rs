use serde::{Deserialize, Serialize, de::Error as _};

use crate::{
    post::Post,
    profile::Profile,
    response::{Response, ResponseRebroadcast},
};

/// All message types supported by the onlyfriends wire format
///
/// # Compatibility
/// The postcard wire format is stable, and describes tagged unions (enums) with a `varint(u32)` discriminant.
/// To maintain cross-compatibility between versions, we explicitly set the discriminant for each variant.
/// A discriminant should never be re-used, therefore removing variants is discouraged.
#[derive(Serialize, Deserialize)]
#[repr(u32)] // postcard uses u32 to tag enums
pub enum Message {
    Post(Post) = 0,
    Profile(Profile) = 1,
    Response(Response) = 2,
    Rebroadcast(ResponseRebroadcast) = 3,
}

/// Unique identifier for a [`Message`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageId(pub [u8; 64]);

// TODO: deduplicate these impls with LetterId
impl<'de> Deserialize<'de> for MessageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: &[u8] = Deserialize::deserialize(deserializer)?;
        Ok(MessageId(bytes.try_into().map_err(|_| {
            D::Error::invalid_length(bytes.len(), &"64")
        })?))
    }
}

impl Serialize for MessageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.0.as_slice();
        bytes.serialize(serializer)
    }
}
