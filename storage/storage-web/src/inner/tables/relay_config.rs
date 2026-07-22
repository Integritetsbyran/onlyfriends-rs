use deli::Model;
use serde::{Deserialize, Serialize};
use storage_common::types::relay_config::RelayConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct WebRelayConfig {
    #[deli(key)]
    id: u32,
    url: String,
}

impl From<RelayConfig> for WebRelayConfig {
    fn from(config: RelayConfig) -> Self {
        Self {
            id: 0, // Hardcoded ID since we only ever store one relay config in the database.
            url: config.url,
        }
    }
}

impl From<WebRelayConfig> for RelayConfig {
    fn from(web: WebRelayConfig) -> Self {
        Self { url: web.url }
    }
}
