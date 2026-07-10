use base64::Engine;
use serde::Deserialize;

pub struct RelayClient {
    base_url: String,
    http: reqwest::Client,
}

impl RelayClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn post_item(&self, addr: &str, item: &[u8]) -> reqwest::Result<()> {
        let item_b64 = base64::engine::general_purpose::STANDARD.encode(item);
        self.http
            .post(format!("{}/mailbox/{addr}", self.base_url))
            .json(&serde_json::json!({"item_b64": item_b64}))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn get_items(
        &self,
        addr: &str,
        after: usize,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct Resp {
            items_b64: Vec<String>,
        }

        let response = self
            .http
            .get(format!("{}/mailbox/{addr}?after={after}", self.base_url))
            .send()
            .await?
            .json::<Resp>()
            .await?;

        let mut out = Vec::with_capacity(response.items_b64.len());
        for s in &response.items_b64 {
            let bytes = base64::engine::general_purpose::STANDARD.decode(s)?;
            out.push(bytes);
        }
        Ok(out)
    }
}
