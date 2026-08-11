use crate::Signable;
use crate::crypto::{self, PublicKeySealed};
use crate::identity::{Identity, PublicIdentity};
use crate::message::Message;
use crate::signing::Signature;
use serde::{Deserialize, Serialize, de::Error as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LetterId([u8; 64]);

impl LetterId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 64]> for LetterId {
    fn from(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Letter {
    pub content_ct: Vec<u8>,
    pub content_nonce: [u8; 24],
    pub sig: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Envelope {
    pub letter: Letter,
    pub sealed_content_key: PublicKeySealed,
}

impl Envelope {
    pub fn open_envelope(
        &self,
        recipient: &Identity,
        author_pub: &PublicIdentity,
    ) -> crate::Result<Message> {
        let content_key = PublicKeySealed::open(&recipient.dh_secret(), &self.sealed_content_key)?;
        let vk = ed25519_dalek::VerifyingKey::try_from(&author_pub.sign_pub)?;
        let sig = ed25519_dalek::Signature::from(self.letter.sig);
        self.letter.verify_signature(&vk, &sig)?;

        let key: [u8; 32] = content_key
            .as_slice()
            .try_into()
            .map_err(|_| crate::Error::BadKey)?;
        let message =
            crypto::aead_decrypt(&key, &self.letter.content_nonce, &self.letter.content_ct)?;

        postcard::from_bytes(&message).map_err(|_| crate::Error::Serialize)
    }

    pub fn seal_envelope(
        author: &Identity,
        payload: &Message,
        recipients: &[PublicIdentity],
    ) -> Vec<Envelope> {
        let post = postcard::to_allocvec(payload).expect("serialization always succeeds");

        let content_key: [u8; 32] = crypto::random_bytes();
        let (body_nonce, body_ct) = crypto::aead_encrypt(&content_key, &post);

        let mut letter = Letter {
            content_ct: body_ct,
            content_nonce: body_nonce,
            sig: Signature::invalid(),
        };
        letter.sig = letter.sign_with(&author.signing_key());

        let mut envelopes: Vec<Envelope> = vec![];
        for r in recipients.iter() {
            let sealed = PublicKeySealed::seal(&r.dh_pub, &content_key);
            envelopes.push(Envelope {
                letter: letter.clone(),
                sealed_content_key: sealed,
            });
        }

        envelopes
    }
}

impl Letter {
    /// Calculate the [`LetterId`] by hashing `self`.
    pub fn id(&self) -> LetterId {
        LetterId(self.signing_hash_bytes())
    }
}

impl Signable for Letter {
    fn signing_bytes(&self) -> Vec<u8> {
        let Self {
            content_ct,
            content_nonce,
            sig: _, /*signature should not contain itself*/
        } = &self;

        postcard::to_allocvec(&(&content_ct, content_nonce)).expect("serializes")
    }
}

impl Serialize for LetterId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LetterId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Using Vec instead of &[u8] here because serde_wasm_bindgen's deserializer doesn't support &[u8] and will fail to deserialize it (not entirely clear why).
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let expected_len = "64"; // sha512 hashes are 64 bytes
        Ok(LetterId(bytes.as_slice().try_into().map_err(|_| {
            D::Error::invalid_length(bytes.len(), &expected_len)
        })?))
    }
}
