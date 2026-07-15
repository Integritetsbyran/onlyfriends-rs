use serde::{Deserialize, Serialize, de::Error as _};

use crate::{
    Error, Identity, PublicIdentity,
    crypto::{self, SealedBox},
    message::Message,
    signing::{Signable, Signed, SignedButUnverified},
};

/// Symmetric encryption key for a [`Letter`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LetterKey(pub [u8; 32]);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipient {
    /// [`UserId`] of the [`Letter`] recipient.
    pub id: SignPub,

    /// Sealed decryption-key for the recipient to decrypt [`Letter::content_ciphertext`].
    pub key: SealedBox<LetterKey>,
}

/// A [`Letter`] with a list of [`Recipient`]s.
#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub letter: SignedButUnverified<Letter>,
    pub recipients: Vec<Recipient>,
}

/// Unique identifier for a [`Letter`] / [`Envelope`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LetterId(pub [u8; 64]);

// TODO: newtype
/// Public signing key that uniquely identifies a user.
pub type SignPub = [u8; 32];

/// An encrypted [`Message`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Letter {
    /// Letter author
    pub author: SignPub,

    /// UNIX timestamp of the message creation time
    pub created_at: u64,

    /// An encrypted [`Message`]
    pub message_ciphertext: Vec<u8>,

    pub message_nonce: [u8; 24],
}

impl LetterId {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }
}

impl Signable for Letter {
    fn signing_bytes(&self) -> Vec<u8> {
        let Letter {
            author,
            created_at,
            message_ciphertext,
            message_nonce,
        } = self;
        postcard::to_allocvec(&(author, created_at, message_ciphertext, message_nonce))
            .expect("serializes")
    }
}

impl Letter {
    pub fn id(&self) -> LetterId {
        // Use sha512 hash of the letter as identifier.
        LetterId(self.pre_hash_bytes())
    }
}

impl Envelope {
    pub fn seal(author: &Identity, message: &Message, recipients: &[PublicIdentity]) -> Envelope {
        let message = postcard::to_allocvec(message).expect("serializes");

        let letter_key = LetterKey(crypto::random_bytes());
        let (message_nonce, message_ciphertext) = crypto::aead_encrypt(&letter_key.0, &message);
        let letter = Letter {
            author: author.public().sign_pub,
            created_at: unix_time(),
            message_ciphertext,
            message_nonce,
        };
        let letter = Signed::new(letter, &author.signing_key()).into();

        let recipients = recipients
            .iter()
            .map(|r| {
                Recipient {
                    id: r.sign_pub, // TODO: is this correct?
                    key: SealedBox::seal(&r.dh_pub, &letter_key.0),
                }
            })
            .collect();

        Envelope { letter, recipients }
    }
}

impl Letter {
    pub fn open(
        &self,
        key: &SealedBox<LetterKey>,
        recipient: &Identity,
        author_pub: &PublicIdentity,
    ) -> crate::Result<Message> {
        let LetterKey(letter_key) = key.open_try_from(&recipient.dh_secret())?;

        let vk = ed25519_dalek::VerifyingKey::from_bytes(&author_pub.sign_pub)
            .map_err(|_| Error::BadKey)?;
        let sig_bytes: [u8; 64] = self
            .sig
            .as_slice()
            .try_into()
            .map_err(|_| Error::Signature)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        vk.verify_strict(&self.signing_bytes(), &sig)
            .map_err(|_| Error::Signature)?;

        let key: [u8; 32] = letter_key
            .as_slice()
            .try_into()
            .map_err(|_| Error::BadKey)?;
        let message = crypto::aead_decrypt(&key, &self.message_nonce, &self.message_ciphertext)?;
        postcard::from_bytes(&message).map_err(|_| Error::Serialize)
    }
}

impl TryFrom<Vec<u8>> for LetterKey {
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(LetterKey(value.try_into()?))
    }
}

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl<'de> Deserialize<'de> for LetterId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: &[u8] = Deserialize::deserialize(deserializer)?;
        Ok(LetterId(bytes.try_into().map_err(|_| {
            D::Error::invalid_length(bytes.len(), &"64")
        })?))
    }
}

impl Serialize for LetterId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.0.as_slice();
        bytes.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use crate::envelope::{Letter, LetterId};
    use rand::{Rng, RngExt};
    use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};

    #[test]
    fn letter_id() {
        let seed = 0x0d013930c969d111; // chosen by fair dice roll
        let mut prng = ChaCha8Rng::seed_from_u64(seed);
        let mut letter = Letter {
            author: prng.random(),
            created_at: prng.random(),
            message_ciphertext: vec![0; 128],
            message_nonce: prng.random(),
        };
        prng.fill_bytes(&mut letter.message_ciphertext);
        let id = letter.id();
        assert_eq!(
            id,
            LetterId([0; 64]),
            "Letter ID must be consistent between software versions",
        );
    }
}
