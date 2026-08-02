use deli::Model;
use keystone::{envelope::LetterId, identity::SigningPublicKey};
use serde::{Deserialize, Serialize};
use storage_common::types::stored_response::{ResponseKind, StoredResponse};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[deli(key(letter_id, author, kind))]
pub struct WebResponse {
    #[deli(index)]
    letter_id: LetterId,
    author: SigningPublicKey,
    kind: ResponseKind,
    content: String,
}

impl WebResponse {
    pub fn new(letter_id: LetterId, response: StoredResponse) -> Self {
        Self {
            letter_id,
            author: response.author,
            kind: response.kind,
            content: response.content,
        }
    }
}

impl From<WebResponse> for StoredResponse {
    fn from(web_response: WebResponse) -> Self {
        Self {
            author: web_response.author,
            kind: web_response.kind,
            content: web_response.content,
        }
    }
}
