use crate::message::Message;
use crate::{Signable};
use crate::crypto::{self, SealedBox};
use crate::identity::{Identity, PublicIdentity, SigningPublicKey};
use crate::signing::Signature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LetterId([u8; 16]);

impl LetterId {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }

    pub fn to_byte_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 16]> for LetterId {
    fn from(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Letter {
    pub id: LetterId,
    pub author: SigningPublicKey,
    pub created_at: u64,
    pub content_ct: Vec<u8>,
    pub content_nonce: [u8; 24],
    pub sig: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Envelope {
    pub post: Letter,
    pub sealed_content_key: SealedBox,
}

impl Envelope {
    pub fn open_envelope(
        &self,
    recipient: &Identity,
    author_pub: &PublicIdentity,
) -> crate::Result<Message> {
    //TODO: Why is env not self??
    let content_key = SealedBox::open(&recipient.dh_secret(), &self.sealed_content_key)?;

    let vk = ed25519_dalek::VerifyingKey::try_from(&author_pub.sign_pub)?;
    let sig = ed25519_dalek::Signature::from(self.post.sig);
    vk.verify_strict(&self.post.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    let key: [u8; 32] = content_key
        .as_slice()
        .try_into()
        .map_err(|_| crate::Error::BadKey)?;
    let post = crypto::aead_decrypt(&key, &self.post.content_nonce, &self.post.content_ct)?;

    postcard::from_bytes(&post).map_err(|_| crate::Error::Serialize)
}

    pub fn seal_envelope(
    author: &Identity,
    payload: &Message,
    recipients: &[PublicIdentity],
) -> Vec<Envelope>
{
    let post = postcard::to_allocvec(payload).expect("serialization always succeeds");

    let content_key: [u8; 32] = crypto::random_bytes();
    let (body_nonce, body_ct) = crypto::aead_encrypt(&content_key, &post);
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut post = Letter {
        id: LetterId::random(),
        author: author.public().sign_pub,
        created_at,
        content_ct: body_ct,
        content_nonce: body_nonce,
        sig: Signature::invalid(),
    };
    post.sig = post.sign_with(&author.signing_key());

    let mut envelopes: Vec<Envelope> = vec![];
    for r in recipients.iter() {
        let sealed = SealedBox::seal(&r.dh_pub, &content_key);
        envelopes.push(Envelope {
            post: post.clone(),
            sealed_content_key: sealed,
        });
    }

    envelopes
}
}

impl Signable for Letter {
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
