use keystone::{identity::SigningPublicKey, response::Response};
use serde::{Deserialize, Serialize};

use crate::storage::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseKind {
    Reaction,
    Comment,
}

impl From<&ResponseKind> for u8 {
    fn from(kind: &ResponseKind) -> Self {
        match kind {
            ResponseKind::Reaction => 0,
            ResponseKind::Comment => 1,
        }
    }
}

impl TryFrom<u8> for ResponseKind {
    type Error = StorageError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ResponseKind::Reaction),
            1 => Ok(ResponseKind::Comment),
            v => Err(StorageError::InvalidResponseKind(v)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredResponse {
    pub author: SigningPublicKey,
    pub kind: ResponseKind,
    pub content: String,
}

impl From<Response> for StoredResponse {
    fn from(value: Response) -> Self {
        let (kind, content) = match value.body {
            keystone::ResponseBody::Comment { text } => (ResponseKind::Comment, text),
            keystone::ResponseBody::Reaction { emoji } => (ResponseKind::Reaction, emoji),
        };

        Self {
            author: value.author,
            kind,
            content,
        }
    }
}
