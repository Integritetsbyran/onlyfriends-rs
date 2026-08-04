use ed25519_dalek::Sha512;
use sha2::{Digest as _, digest::FixedOutput as _};

use crate::{crypto::CRYPTO_CTX, signing::Signature};

/// An object that can be cryptographically signed for authenticity.
pub trait Signable {
    /// Create a byte-representation of `self` that can be used for signing.
    fn signing_bytes(&self) -> Vec<u8>;

    /// Compute the [`Sha512`] hash of [`Self::signing_bytes`].
    fn signing_hash(&self) -> Sha512 {
        let mut hash: Sha512 = Sha512::new();
        hash.update(self.signing_bytes());
        hash
    }

    /// Compute the [`Sha512`] hash of [`Self::signing_bytes`] and return it as bytes.
    fn signing_hash_bytes(&self) -> [u8; 64] {
        *self.signing_hash().finalize_fixed().as_ref()
    }

    /// Sign [`Self::signing_hash`] with `key`.
    fn sign_with(&self, key: &ed25519_dalek::SigningKey) -> Signature {
        key.sign_prehashed(self.signing_hash(), Some(CRYPTO_CTX))
            .expect("valid ctx")
            .into()
    }

    /// Verify a signature against `self`.
    fn verify_signature(
        &self,
        vk: &ed25519_dalek::VerifyingKey,
        sig: &ed25519_dalek::Signature,
    ) -> crate::Result<()> {
        vk.verify_prehashed_strict(self.signing_hash(), Some(CRYPTO_CTX), sig)
            .map_err(|_| crate::Error::Signature)
    }
}

impl<T: Signable> Signable for &T {
    fn signing_bytes(&self) -> Vec<u8> {
        <T as Signable>::signing_bytes(self)
    }
}

impl<T: Signable> Signable for &mut T {
    fn signing_bytes(&self) -> Vec<u8> {
        <T as Signable>::signing_bytes(self)
    }
}
