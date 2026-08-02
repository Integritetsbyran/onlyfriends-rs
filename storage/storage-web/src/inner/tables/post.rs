use deli::Model;
use keystone::{envelope::LetterId, identity::SigningPublicKey, post::PostContent};
use serde::{Deserialize, Serialize};
use storage_common::types::stored_post::StoredPost;

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct WebPost {
    #[deli(key)]
    post_id: LetterId,
    author: SigningPublicKey,
    body: String,
    created_at: u64,
    media: Vec<keystone::media::Media>,
}

impl WebPost {
    pub fn new(encrypted: keystone::Letter, post: PostContent) -> Self {
        Self {
            post_id: encrypted.id,
            author: encrypted.author,
            body: post.body,
            created_at: encrypted.created_at,
            media: post.media,
        }
    }
}

impl From<WebPost> for StoredPost {
    fn from(web_post: WebPost) -> Self {
        Self {
            id: web_post.post_id,
            author: web_post.author,
            body: web_post.body,
            created_at: web_post.created_at,
            media: web_post.media,
        }
    }
}
