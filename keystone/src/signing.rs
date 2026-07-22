use std::ops::Deref;

use serde::{Deserialize, Serialize, de::Error as _};

/// Wrapper around [`ed25519_dalek::Signature`] that implements [`Serialize`] and [`Deserialize`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature {
    inner: ed25519_dalek::Signature,
}

impl Signature {
    /// Create an invalid signature.
    pub fn invalid() -> Self {
        Self::from([0u8; 64])
    }
}

impl From<[u8; 64]> for Signature {
    fn from(bytes: [u8; 64]) -> Self {
        ed25519_dalek::Signature::from(bytes).into()
    }
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(inner: ed25519_dalek::Signature) -> Self {
        Self { inner }
    }
}

impl From<Signature> for ed25519_dalek::Signature {
    fn from(signature: Signature) -> Self {
        signature.inner
    }
}

impl Deref for Signature {
    type Target = ed25519_dalek::Signature;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.inner.to_bytes();
        bytes.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Using Vec instead of &[u8] here because serde_wasm_bindgen's deserializer doesn't support &[u8] and will fail to deserialize it (not entirely clear why).
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let expected_len = "64"; // ed25519 signatures are 64 bytes
        Ok(Signature {
            inner: bytes
                .as_slice()
                .try_into()
                .map_err(|_| D::Error::invalid_length(bytes.len(), &expected_len))?,
        })
    }
}
