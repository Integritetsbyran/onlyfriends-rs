use serde::{Deserialize, Serialize};

use crate::Profile;
use crate::post::PostContent;
use crate::response::{ResponseInner, ResponseRebroadcast};

/// All message types supported by the onlyfriends wire format
///
/// # Compatibility
/// The postcard wire format is stable, and describes tagged unions (enums) with a `varint(u32)` discriminant.
/// To maintain cross-compatibility between versions, we explicitly set the discriminant for each variant.
/// A discriminant should never be re-used, therefore removing variants is discauraged.
#[derive(Serialize, Deserialize)]
#[repr(u32)] // postcard uses u32 to tag enums
pub enum Message {
    Post(PostContent) = 0,
    Profile(Profile) = 1,
    Response(ResponseInner) = 2,
    Rebroadcast(ResponseRebroadcast) = 3,
}
