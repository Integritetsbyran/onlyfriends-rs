use crate::{crypto, labels};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterSeed([u8; 32]);

impl MasterSeed {
    pub fn random() -> Self {
        Self(crypto::random_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl From<[u8; 32]> for MasterSeed {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningPublicKey([u8; 32]);

impl From<[u8; 32]> for SigningPublicKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&SigningPublicKey> for ed25519_dalek::VerifyingKey {
    type Error = crate::Error;

    fn try_from(value: &SigningPublicKey) -> Result<Self, Self::Error> {
        ed25519_dalek::VerifyingKey::from_bytes(&value.0).map_err(|_| crate::Error::BadKey)
    }
}

impl SigningPublicKey {
    pub fn to_byte_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    pub fn to_short_hex(&self) -> String {
        self.0.iter().take(8).fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd)]
pub struct DhPublicKey([u8; 32]);

impl DhPublicKey {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl From<[u8; 32]> for DhPublicKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<&x25519_dalek::StaticSecret> for DhPublicKey {
    fn from(secret: &x25519_dalek::StaticSecret) -> Self {
        Self(x25519_dalek::PublicKey::from(secret).to_bytes())
    }
}

impl From<&DhPublicKey> for x25519_dalek::PublicKey {
    fn from(pubkey: &DhPublicKey) -> Self {
        x25519_dalek::PublicKey::from(pubkey.0)
    }
}

#[derive(Clone)]
pub struct Identity {
    pub master_seed: MasterSeed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub sign_pub: SigningPublicKey,
    pub dh_pub: DhPublicKey,
}

impl Identity {
    pub fn generate() -> Identity {
        Identity {
            master_seed: MasterSeed::random(),
        }
    }

    pub fn from_seed(master_seed: MasterSeed) -> Identity {
        Identity { master_seed }
    }

    pub fn public(&self) -> PublicIdentity {
        let sign_pub = self.signing_key().verifying_key().to_bytes().into();
        let dh_pub = DhPublicKey::from(&self.dh_secret());
        PublicIdentity { sign_pub, dh_pub }
    }

    pub fn signing_key(&self) -> ed25519_dalek::SigningKey {
        let bytes = crypto::derive32(&self.master_seed.0, labels::IDENTITY_ED25519);
        ed25519_dalek::SigningKey::from_bytes(&bytes)
    }

    pub fn dh_secret(&self) -> x25519_dalek::StaticSecret {
        let bytes = crypto::derive32(&self.master_seed.0, labels::IDENTITY_X25519);
        x25519_dalek::StaticSecret::from(bytes)
    }
}

impl PublicIdentity {
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("serializes")
    }
}

impl TryFrom<&[u8]> for PublicIdentity {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> crate::Result<Self> {
        postcard::from_bytes(value).map_err(|_| crate::Error::Serialize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_identity() {
        let seed = MasterSeed::random();
        assert_eq!(
            Identity::from_seed(seed).public(),
            Identity::from_seed(seed).public()
        )
    }

    #[test]
    fn id_survives_qr_round_trip() {
        let id = Identity::generate().public();
        assert_eq!(PublicIdentity::try_from(&id.to_bytes()[..]).unwrap(), id)
    }
}
