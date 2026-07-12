use serde::{Deserialize, Serialize};

use crate::post::SealedPost;
use crate::crypto::SealedBox;

#[derive(Serialize, Deserialize)]
pub enum Envelope {
    Post(SealedPost),
    Profile(SealedBox),
    Response(SealedBox),
    Rebroadcast(SealedBox),
}
