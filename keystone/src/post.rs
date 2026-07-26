use crate::Signable;
use crate::crypto::{self, SealedBox};
use crate::identity::{Identity, PublicIdentity, SigningPublicKey};
use crate::media::Media;
use crate::signing::Signature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostId([u8; 16]);

impl PostId {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }

    pub fn to_byte_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for PostId {
    fn from(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostContent {
    /// The plain-text content of a post.
    pub body: String,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedPost {
    pub id: PostId,
    pub author: SigningPublicKey,
    pub created_at: u64,
    pub content_ct: Vec<u8>,
    pub content_nonce: [u8; 24],
    pub sig: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealedPost {
    pub post: EncryptedPost,
    pub sealed_content_key: SealedBox,
}

impl PostContent {
    pub fn from_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            media: Default::default(),
        }
    }
}

impl Signable for EncryptedPost {
    fn signing_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(&(
            self.id,
            self.author,
            self.created_at,
            &self.content_ct,
            self.content_nonce,
        ))
        .expect("serializes")
    }
}

pub fn seal_post(
    author: &Identity,
    post: &PostContent,
    recipients: &[PublicIdentity],
) -> Vec<SealedPost> {
    use ed25519_dalek::Signer;

    let post = postcard::to_allocvec(post).expect("serialization always succeeds");

    let content_key: [u8; 32] = crypto::random_bytes();
    let (body_nonce, body_ct) = crypto::aead_encrypt(&content_key, &post);
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut post = EncryptedPost {
        id: PostId::random(),
        author: author.public().sign_pub,
        created_at,
        content_ct: body_ct,
        content_nonce: body_nonce,
        sig: Signature::invalid(),
    };
    let sig = author.signing_key().sign(&post.signing_bytes());
    post.sig = sig.into();

    let mut envelopes: Vec<SealedPost> = vec![];
    for r in recipients.iter() {
        let sealed = SealedBox::seal(&r.dh_pub, &content_key);
        envelopes.push(SealedPost {
            post: post.clone(),
            sealed_content_key: sealed,
        });
    }

    envelopes
}

pub fn open_post(
    recipient: &Identity,
    author_pub: &PublicIdentity,
    env: &SealedPost,
) -> crate::Result<PostContent> {
    let content_key = SealedBox::open(&recipient.dh_secret(), &env.sealed_content_key)?;

    let vk = ed25519_dalek::VerifyingKey::try_from(&author_pub.sign_pub)?;
    let sig = ed25519_dalek::Signature::from(env.post.sig);
    vk.verify_strict(&env.post.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    let key: [u8; 32] = content_key
        .as_slice()
        .try_into()
        .map_err(|_| crate::Error::BadKey)?;
    let post = crypto::aead_decrypt(&key, &env.post.content_nonce, &env.post.content_ct)?;

    postcard::from_bytes(&post).map_err(|_| crate::Error::Serialize)
}
