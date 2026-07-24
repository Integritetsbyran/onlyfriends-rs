use keystone::{identity::SigningPublicKey, media::Media, post::PostId};

pub struct StoredPost {
    pub id: PostId,
    pub author: SigningPublicKey,
    pub body: String, // decrypted
    pub created_at: u64,
    pub media: Vec<Media>,
}
