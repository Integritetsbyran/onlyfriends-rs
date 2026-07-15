use std::marker::PhantomData;

use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::PublicKey;

use crate::labels;

pub const CRYPTO_CTX: &[u8] = b"onlyfriends";

/// N fresh random bytes from the OS CSPRNG.
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    rand::rng().fill_bytes(&mut out);
    out
}

/// Derive 32 bytes from input keying material + a label. This is THE pattern.
pub fn derive32(ikm: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, ikm); // None = no salt; label separates
    let mut out = [0u8; 32];
    hk.expand(info, &mut out)
        .expect("32 is a valid HKDF length");
    out
}

/// Encrypt under `key`; returns (random nonce, ciphertext).
pub fn aead_encrypt(key: &[u8; 32], plaintext: &[u8]) -> ([u8; 24], Vec<u8>) {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let nonce: [u8; 24] = random_bytes();
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .expect("AEAD encryption doesn't fail with valid sizes");
    (nonce, ct)
}

pub fn aead_decrypt(key: &[u8; 32], nonce: &[u8; 24], ciphertext: &[u8]) -> crate::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|_| crate::Error::Aead)
}

/// An assymetrically encrypted `Vec<u8>`.
///
/// Use [`SealedBox::seal`] to encrypt a payload of bytes.
/// Use [`SealedBox::open`] to decrypt it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedBox<T = Vec<u8>> {
    pub ephemeral_pub: [u8; 32],
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<T> SealedBox<T> {
    pub fn seal(recipient_dh_pub: &[u8; 32], cleartext: &[u8]) -> Self {
        let eph = x25519_dalek::EphemeralSecret::random_from_rng(&mut rand::rng());
        let e_pub = x25519_dalek::PublicKey::from(&eph);
        let shared = eph.diffie_hellman(&x25519_dalek::PublicKey::from(*recipient_dh_pub));
        let info = [labels::SEAL, &e_pub.to_bytes(), recipient_dh_pub].concat();
        let wrap_key = derive32(&shared.to_bytes(), &info);
        let (nonce, ciphertext) = aead_encrypt(&wrap_key, cleartext);
        SealedBox {
            ephemeral_pub: e_pub.to_bytes(),
            nonce,
            ciphertext,
            _phantom: PhantomData,
        }
    }

    pub fn open(&self, recipient_dh_secret: &x25519_dalek::StaticSecret) -> crate::Result<Vec<u8>> {
        let shared = recipient_dh_secret.diffie_hellman(&PublicKey::from(self.ephemeral_pub));
        let info = [
            labels::SEAL,
            &self.ephemeral_pub,
            &PublicKey::from(recipient_dh_secret).to_bytes(),
        ]
        .concat();
        let wrap_key = derive32(&shared.to_bytes(), &info);
        aead_decrypt(&wrap_key, &self.nonce, &self.ciphertext)
    }
}

impl<T: TryFrom<Vec<u8>>> SealedBox<T> {
    pub fn open_try_from(
        &self,
        recipient_dh_secret: &x25519_dalek::StaticSecret,
    ) -> crate::Result<T> {
        let bytes = self.open(recipient_dh_secret)?;
        bytes.try_into().map_err(|_| crate::Error::Serialize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aead_round_trips_and_rejects_tampering() {
        let key: [u8; 32] = random_bytes();
        let (nonce, mut ct) = aead_encrypt(&key, b"hello");
        assert_eq!(aead_decrypt(&key, &nonce, &ct).unwrap(), b"hello");
        ct[0] ^= 0xff; // flip a byte
        assert!(aead_decrypt(&key, &nonce, &ct).is_err()); // tamper detected
    }

    #[test]
    fn seal_open_round_trip() {
        use x25519_dalek::{PublicKey, StaticSecret};
        let sk = StaticSecret::from(random_bytes::<32>());
        let pk = PublicKey::from(&sk).to_bytes();
        let secret = b"a content key goes here, 32 byte";
        let sealed = SealedBox::<Vec<u8>>::seal(&pk, secret);
        assert_eq!(sealed.open(&sk).unwrap(), secret);
    }
}
