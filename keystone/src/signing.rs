use std::ops::Deref;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512, digest::FixedOutput};

use crate::crypto::CRYPTO_CTX;

#[derive(Serialize, Deserialize)]
pub struct SignedButUnverified<T> {
    #[serde(flatten)]
    pub signed: T,
    pub sig: Vec<u8>,
}

pub struct Signed<T> {
    inner: SignedButUnverified<T>,
}

impl<T> From<Signed<T>> for SignedButUnverified<T> {
    fn from(signed: Signed<T>) -> Self {
        signed.inner
    }
}

impl<T: Signable> Signed<T> {
    pub fn new(unsigned: T, key: &ed25519_dalek::SigningKey) -> Self {
        let pre_hash = unsigned.pre_hash();
        let sig = key.sign_prehashed(pre_hash, Some(CRYPTO_CTX)).unwrap();
        let sig = sig.to_bytes().to_vec();
        Self {
            inner: SignedButUnverified {
                signed: unsigned,
                sig,
            },
        }
    }

    pub fn sig(&self) -> &[u8] {
        &self.inner.sig
    }
}

impl<T> Deref for Signed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.signed
    }
}

impl<T> Deref for SignedButUnverified<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.signed
    }
}

pub trait Signable {
    fn signing_bytes(&self) -> Vec<u8>;

    fn pre_hash(&self) -> Sha512 {
        let mut hash: Sha512 = Sha512::new();
        hash.update(&self.signing_bytes());
        hash
    }

    fn pre_hash_bytes(&self) -> [u8; 64] {
        *self.pre_hash().finalize_fixed().as_ref()
    }
}

impl<T: Signable> Signable for &T {
    fn signing_bytes(&self) -> Vec<u8> {
        <T as Signable>::signing_bytes(self)
    }
}
