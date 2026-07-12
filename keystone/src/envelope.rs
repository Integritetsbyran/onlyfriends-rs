use serde::{Deserialize, Serialize};

use crate::crypto::SealedBox;
use crate::post::SealedPost;

#[derive(Serialize, Deserialize)]
pub enum Envelope {
    Post(SealedPost),
    Profile(SealedBox),
    Response(SealedBox),
    Rebroadcast(SealedBox),
}
