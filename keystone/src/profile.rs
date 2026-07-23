use serde::{Deserialize, Serialize};

use crate::{Identity, identity::SigningPublicKey};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub owner: SigningPublicKey,
    pub display_name: String,
    pub bio: String,
    pub version: u64,
    pub sig: Vec<u8>,
}

impl Profile {
    pub fn signing_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(&(self.owner, &self.display_name, &self.bio, &self.version))
            .expect("serializes")
    }
}

pub fn create_profile(me: &Identity, display_name: &str, bio: &str, version: u64) -> Profile {
    use ed25519_dalek::Signer;

    let mut profile = Profile {
        owner: me.public().sign_pub,
        display_name: display_name.to_string(),
        bio: bio.to_string(),
        version,
        sig: vec![],
    };

    let sig = me.signing_key().sign(&profile.signing_bytes());
    profile.sig = sig.to_bytes().to_vec();

    profile
}

pub fn verify_profile(p: &Profile) -> crate::Result<()> {
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&p.owner).map_err(|_| crate::Error::BadKey)?;

    let sig_bytes: [u8; 64] = p
        .sig
        .as_slice()
        .try_into()
        .map_err(|_| crate::Error::Signature)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

    vk.verify_strict(&p.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    Ok(())
}
