use std::fmt;

use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Media {
    #[serde_as(as = "DisplayFromStr")]
    pub mime: Mime,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Media {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Media")
            .field("mime", &self.mime)
            .finish_non_exhaustive()
    }
}
