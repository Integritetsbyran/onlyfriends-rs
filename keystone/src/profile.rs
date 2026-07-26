use serde::{Deserialize, Serialize};

use crate::{Identity, Signable, identity::SigningPublicKey, signing::Signature};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub owner: SigningPublicKey,
    pub display_name: String,
    pub bio: String,
    pub version: u64,
    pub sig: Signature,
}

impl Signable for Profile {
    fn signing_bytes(&self) -> Vec<u8> {
        let Self {
            owner,
            display_name,
            bio,
            version,
            sig: _, // don't sign the signature
        } = self;
        postcard::to_allocvec(&(owner, display_name, bio, version)).expect("serializes")
    }
}

pub fn create_profile(me: &Identity, display_name: &str, bio: &str, version: u64) -> Profile {
    let mut profile = Profile {
        owner: me.public().sign_pub,
        display_name: display_name.to_string(),
        bio: bio.to_string(),
        version,
        sig: Signature::invalid(),
    };
    profile.sig = profile.sign_with(&me.signing_key());
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
