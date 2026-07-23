use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::PublicKey;

use crate::{identity::DhPublicKey, labels};

/// N fresh random bytes from the OS CSPRNG.
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    OsRng.fill_bytes(&mut out);
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealedBox {
    pub ephemeral_pub: [u8; 32],
    pub nonce: [u8; 24],
    pub ct: Vec<u8>,
}

impl SealedBox {
    pub fn seal(recipient_dh_pub: &DhPublicKey, secret: &[u8]) -> SealedBox {
        let eph = x25519_dalek::EphemeralSecret::random_from_rng(OsRng);
        let e_pub = x25519_dalek::PublicKey::from(&eph);
        let shared = eph.diffie_hellman(&x25519_dalek::PublicKey::from(*recipient_dh_pub));
        let info = [labels::SEAL, &e_pub.to_bytes(), recipient_dh_pub].concat();
        let wrap_key = derive32(&shared.to_bytes(), &info);
        let (nonce, ct) = aead_encrypt(&wrap_key, secret);
        SealedBox {
            ephemeral_pub: e_pub.to_bytes(),
            nonce,
            ct,
        }
    }

    pub fn open(
        recipient_dh_secret: &x25519_dalek::StaticSecret,
        sealed: &SealedBox,
    ) -> crate::Result<Vec<u8>> {
        let shared = recipient_dh_secret.diffie_hellman(&PublicKey::from(sealed.ephemeral_pub));
        let info = [
            labels::SEAL,
            &sealed.ephemeral_pub,
            &PublicKey::from(recipient_dh_secret).to_bytes(),
        ]
        .concat();
        let wrap_key = derive32(&shared.to_bytes(), &info);
        aead_decrypt(&wrap_key, &sealed.nonce, &sealed.ct)
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
        let sealed = SealedBox::seal(&pk, secret);
        assert_eq!(SealedBox::open(&sk, &sealed).unwrap(), secret);
    }
}
