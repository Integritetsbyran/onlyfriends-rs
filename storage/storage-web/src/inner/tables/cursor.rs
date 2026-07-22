use deli::Model;
use keystone::identity::SigningPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[deli(key(friend, direction, epoch))]
pub struct WebCursor {
    friend: SigningPublicKey,
    direction: u8,
    epoch: u64,
    pub last_index: usize,
}

impl WebCursor {
    pub fn new(friend: SigningPublicKey, direction: u8, epoch: u64, last_index: usize) -> Self {
        Self {
            friend,
            direction,
            epoch,
            last_index,
        }
    }
}
