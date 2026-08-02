use crate::Signable;
use crate::crypto::{self, SealedBox};
use crate::identity::{Identity, PublicIdentity, SigningPublicKey};
use crate::media::Media;
use crate::signing::Signature;
use onlyfriends_time::seconds_since_epoch;
use serde::{Deserialize, Serialize};

use crate::media::Media;

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostContent {
    /// The plain-text content of a post.
    pub body: String,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
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
    let post = postcard::to_allocvec(post).expect("serialization always succeeds");

    let content_key: [u8; 32] = crypto::random_bytes();
    let (body_nonce, body_ct) = crypto::aead_encrypt(&content_key, &post);
    let created_at = seconds_since_epoch();

    let mut post = EncryptedPost {
        id: PostId::random(),
        author: author.public().sign_pub,
        created_at,
        content_ct: body_ct,
        content_nonce: body_nonce,
        sig: Signature::invalid(),
    };
    post.sig = post.sign_with(&author.signing_key());

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
