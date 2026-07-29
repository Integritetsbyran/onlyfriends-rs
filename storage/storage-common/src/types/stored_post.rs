use keystone::{identity::SigningPublicKey, media::Media, envelope::LetterId};

pub struct StoredPost {
    pub id: LetterId,
    pub author: SigningPublicKey,
    pub body: String, // decrypted
    pub created_at: u64,
    pub media: Vec<Media>,
}
