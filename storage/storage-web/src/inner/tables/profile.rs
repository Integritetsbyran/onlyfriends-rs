use deli::Model;
use keystone::{identity::SigningPublicKey, signing::Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct WebProfile {
    #[deli(key)]
    pub owner: SigningPublicKey,
    pub display_name: String,
    pub bio: String,
    pub version: u64,
    pub sig: Signature,
}

impl From<keystone::Profile> for WebProfile {
    fn from(profile: keystone::Profile) -> Self {
        WebProfile {
            owner: profile.owner,
            display_name: profile.display_name,
            bio: profile.bio,
            version: profile.version,
            sig: profile.sig,
        }
    }
}

impl From<WebProfile> for keystone::Profile {
    fn from(web_profile: WebProfile) -> Self {
        keystone::Profile {
            owner: web_profile.owner,
            display_name: web_profile.display_name,
            bio: web_profile.bio,
            version: web_profile.version,
            sig: web_profile.sig,
        }
    }
}
