use deli::Model;
use keystone::{Identity, identity::MasterSeed};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct WebIdentity {
    #[deli(key)]
    id: u32,
    master_seed: MasterSeed,
}

impl From<MasterSeed> for WebIdentity {
    fn from(seed: MasterSeed) -> Self {
        WebIdentity {
            id: 0,
            master_seed: seed,
        }
    }
}

impl From<WebIdentity> for Identity {
    fn from(web_identity: WebIdentity) -> Self {
        Identity::from_seed(web_identity.master_seed)
    }
}
