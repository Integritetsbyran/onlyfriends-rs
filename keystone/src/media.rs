use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Media {
    #[serde_as(as = "DisplayFromStr")]
    pub mime: Mime,
    pub bytes: Vec<u8>,
}
