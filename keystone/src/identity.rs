use crate::{crypto, labels};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Identity {
    pub master_seed: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub sign_pub: [u8; 32],
    pub dh_pub: [u8; 32],
}

impl Identity {
    pub fn generate() -> Identity {
        Identity {
            master_seed: crypto::random_bytes(),
        }
    }

    pub fn from_seed(master_seed: [u8; 32]) -> Identity {
        Identity { master_seed }
    }

    pub fn public(&self) -> PublicIdentity {
        let sign_pub = self.signing_key().verifying_key().to_bytes();
        let dh_pub = x25519_dalek::PublicKey::from(&self.dh_secret()).to_bytes();
        PublicIdentity { sign_pub, dh_pub }
    }

    pub fn signing_key(&self) -> ed25519_dalek::SigningKey {
        let bytes = crypto::derive32(&self.master_seed, labels::IDENTITY_ED25519);
        ed25519_dalek::SigningKey::from_bytes(&bytes)
    }

    pub fn dh_secret(&self) -> x25519_dalek::StaticSecret {
        let bytes = crypto::derive32(&self.master_seed, labels::IDENTITY_X25519);
        x25519_dalek::StaticSecret::from(bytes)
    }
}

impl PublicIdentity {
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> crate::Result<PublicIdentity> {
        postcard::from_bytes(bytes).map_err(|_| crate::Error::Serialize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_identity() {
        let seed = crypto::random_bytes();
        assert_eq!(
            Identity::from_seed(seed).public(),
            Identity::from_seed(seed).public()
        )
    }

    #[test]
    fn id_survives_qr_round_trip() {
        let id = Identity::generate().public();
        assert_eq!(PublicIdentity::from_bytes(&id.to_bytes()).unwrap(), id)
    }
}
