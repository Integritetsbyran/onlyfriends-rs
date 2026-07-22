use deli::Model;
use keystone::{identity::SigningPublicKey, post::PostId};
use serde::{Deserialize, Serialize};
use storage_common::types::stored_response::{ResponseKind, StoredResponse};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[deli(key(post_id, author, kind))]
pub struct WebResponse {
    #[deli(index)]
    post_id: PostId,
    author: SigningPublicKey,
    kind: ResponseKind,
    content: String,
}

impl WebResponse {
    pub fn new(post_id: PostId, response: StoredResponse) -> Self {
        Self {
            post_id,
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
