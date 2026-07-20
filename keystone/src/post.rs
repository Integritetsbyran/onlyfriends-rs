use crate::Error;
use crate::crypto::{self, SealedBox};
use crate::identity::{Identity, PublicIdentity};
use crate::media::Media;
use crate::untyped::{UntypedValue, UntypedValueRef};
use serde::{Deserialize, Serialize};

/// The content of a post
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PostContent {
    /// The plain-text content of a post.
    pub body: String,

    /// Attached pictures and videos.
    pub media: Vec<Media>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PostId(pub [u8; 16]);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedPost {
    pub id: PostId,
    pub author: [u8; 32],
    pub created_at: u64,
    pub content_ct: Vec<u8>,
    pub content_nonce: [u8; 24],
    pub sig: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealedPost {
    pub post: EncryptedPost,
    pub sealed_content_key: SealedBox,
}

impl PostId {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }
}

impl PostContent {
    pub fn from_body(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            media: Default::default(),
        }
    }
}

impl EncryptedPost {
    pub fn signing_bytes(&self) -> Vec<u8> {
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
    let post = UntypedValue {
        version: "1",
        value: post,
    };
    let post = postcard::to_allocvec(&post).expect("serialization always succeeds");

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
        sig: vec![],
    };
    let sig = author.signing_key().sign(&post.signing_bytes());
    post.sig = sig.to_bytes().to_vec();

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
    let post = crypto::aead_decrypt(&key, &env.post.content_nonce, &env.post.content_ct)?;
    let UntypedValueRef { version, value } =
        postcard::from_bytes(&post).map_err(|_| Error::Serialize)?;

    let "1" = version else {
        return Err(Error::Serialize);
    };

    postcard::from_bytes(value).map_err(|_| crate::Error::Serialize)
}
