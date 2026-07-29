use keystone::{identity::SigningPublicKey, media::Media, envelope::PostId};

pub struct StoredPost {
    pub id: PostId,
    pub author: SigningPublicKey,
    pub body: String, // decrypted
    pub created_at: u64,
    pub media: Vec<Media>,
}
