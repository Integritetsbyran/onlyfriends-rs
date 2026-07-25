use serde::{Deserialize, Serialize};

use crate::{Identity, identity::SigningPublicKey, signing::Signature};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub owner: SigningPublicKey,
    pub display_name: String,
    pub bio: String,
    pub version: u64,
    pub sig: Signature,
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
        sig: Signature::invalid(),
    };

    let sig = me.signing_key().sign(&profile.signing_bytes());
    profile.sig = sig.into();

    profile
}

pub fn verify_profile(p: &Profile) -> crate::Result<()> {
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&p.owner.to_bytes())
        .map_err(|_| crate::Error::BadKey)?;

    let sig = ed25519_dalek::Signature::from(p.sig);

    vk.verify_strict(&p.signing_bytes(), &sig)
        .map_err(|_| crate::Error::Signature)?;

    Ok(())
}
