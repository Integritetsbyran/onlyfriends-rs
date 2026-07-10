use crate::crypto::{self, SealedBox};
use crate::identity::{Identity, PublicIdentity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: [u8; 16],
    pub author: [u8; 32],
    pub created_at: u64,
    pub body_ct: Vec<u8>,
    pub body_nonce: [u8; 24],
    pub sig: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostEnvelope {
    pub post: Post,
    pub sealed_content_key: SealedBox,
}

impl Post {
    pub fn signing_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(&(
            self.id,
            self.author,
            self.created_at,
            &self.body_ct,
            self.body_nonce,
        ))
        .expect("serializes")
    }
}

pub fn create_post(
    author: &Identity,
    body: &str,
    recipients: &[PublicIdentity],
) -> Vec<PostEnvelope> {
    use ed25519_dalek::Signer;

    let content_key: [u8; 32] = crypto::random_bytes();
    let (body_nonce, body_ct) = crypto::aead_encrypt(&content_key, body.as_bytes());
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut post = Post {
        id: crypto::random_bytes(),
        author: author.public().sign_pub,
        created_at,
        body_ct,
        body_nonce,
        sig: vec![],
    };
    let sig = author.signing_key().sign(&post.signing_bytes());
    post.sig = sig.to_bytes().to_vec();

    let mut envelopes: Vec<PostEnvelope> = vec![];
    for r in recipients.iter() {
        let sealed = SealedBox::seal(&r.dh_pub, &content_key);
        envelopes.push(PostEnvelope {
            post: post.clone(),
            sealed_content_key: sealed,
        });
    }

    envelopes
}

pub fn open_post(
    recipient: &Identity,
    author_pub: &PublicIdentity,
    env: &PostEnvelope,
) -> crate::Result<String> {
    let content_key = SealedBox::open(&recipient.dh_secret(), &env.sealed_content_key)?;

    let vk = ed25519_dalek::VerifyingKey::from_bytes(&author_pub.sign_pub)
        .map_err(|_| crate::Error::BadKey)?;
    let sig_bytes: [u8; 64] = env
        .post
        .sig
        .as_slice()
        .try_into()
        .map_err(|_| crate::Error::Signature)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    vk.verify_strict(&env.post.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    let key: [u8; 32] = content_key
        .as_slice()
        .try_into()
        .map_err(|_| crate::Error::BadKey)?;
    let body = crypto::aead_decrypt(&key, &env.post.body_nonce, &env.post.body_ct)?;

    String::from_utf8(body).map_err(|_| crate::Error::Serialize)
}
