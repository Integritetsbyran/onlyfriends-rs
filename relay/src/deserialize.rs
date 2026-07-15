use axum::{
    body::{self, Body},
    extract::FromRequest,
    http::{HeaderValue, Response, StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
};
use serde::{Serialize, de::DeserializeOwned};

/// Maximum size of a postcard payload, in bytes.
const CONTENT_LIMIT: usize = 1000_0000_0000; // 1GB

const POSTCARD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/postcard");

/// Deserialize a request body from [`postcard`] into a `T`.
pub struct Postcard<T>(pub T);

impl<S, T> FromRequest<S> for Postcard<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = StatusCode;

    async fn from_request(req: axum::extract::Request, _: &S) -> Result<Self, Self::Rejection> {
        let content_type = req.headers().get(CONTENT_TYPE);
        if content_type != Some(&POSTCARD_CONTENT_TYPE) {
            tracing::warn!("Invalid content-type");
            return Err(StatusCode::BAD_REQUEST);
        }

        let body = body::to_bytes(req.into_body(), CONTENT_LIMIT)
            .await
            .inspect_err(|e| tracing::warn!("{e}"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let value = postcard::from_bytes(&body)
            .inspect_err(|e| tracing::warn!("{e}"))
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        Ok(Postcard(value))
    }
}

impl<T> IntoResponse for Postcard<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let serialized = postcard::to_allocvec(&self.0).expect("T is serializable");
        let body = Body::from(serialized);
        Response::builder()
            .header(CONTENT_TYPE, POSTCARD_CONTENT_TYPE)
            .body(body)
            .expect("Response is valid")
    }
}
