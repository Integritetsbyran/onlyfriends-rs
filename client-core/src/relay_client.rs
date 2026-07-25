use reqwest::header::CONTENT_TYPE;
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum RelayClientError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Postcard (de)serialize error: {0}")]
    Postcard(#[from] postcard::Error),
}

pub type RelayClientResult<T> = Result<T, RelayClientError>;

pub struct RelayClient {
    base_url: String,
    http: reqwest::Client,
}

const POSTCARD_CONTENT_TYPE: &str = "application/postcard";

impl RelayClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Serialize `item` as postcard and post it to the mailbox at `addr`.
    pub async fn post_item(&self, addr: &str, item: &impl Serialize) -> RelayClientResult<()> {
        let item = postcard::to_allocvec(item).expect("serialize");

        self.http
            .post(format!("{}/mailbox/{addr}", self.base_url))
            .header(CONTENT_TYPE, POSTCARD_CONTENT_TYPE)
            .body(item)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn get_items(&self, addr: &str, after: usize) -> RelayClientResult<Vec<Vec<u8>>> {
        let response = self
            .http
            .get(format!("{}/mailbox/{addr}?after={after}", self.base_url))
            .send()
            .await?
            .bytes()
            .await?;

        Ok(postcard::from_bytes(&response)?)
    }
}
