use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::post,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Default)]
struct Store {
    mailboxes: HashMap<String, Vec<Vec<u8>>>,
}

type SharedStore = Arc<Mutex<Store>>;

#[derive(Deserialize)]
struct PostItemRequest {
    item_b64: String,
}

async fn post_mailbox(
    State(store): State<SharedStore>,
    Path(addr): Path<String>,
    Json(req): Json<PostItemRequest>,
) -> Result<(), axum::http::StatusCode> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.item_b64)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let mut store = store.lock().unwrap();
    store.mailboxes.entry(addr).or_default().push(bytes);
    Ok(())
}

#[derive(Serialize)]
struct GetItemsResponse {
    items_b64: Vec<String>,
}

#[derive(Deserialize)]
struct AfterQuery {
    #[serde(default)]
    after: usize,
}

async fn get_mailbox(
    State(store): State<SharedStore>,
    Path(addr): Path<String>,
    Query(q): Query<AfterQuery>,
) -> Json<GetItemsResponse> {
    use base64::Engine;

    let store = store.lock().unwrap();
    let items: &[Vec<u8>] = store
        .mailboxes
        .get(&addr)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let items_b64 = items
        .get(q.after..)
        .unwrap_or(&[])
        .iter()
        .map(|item| base64::engine::general_purpose::STANDARD.encode(item))
        .collect();

    Json(GetItemsResponse { items_b64 })
}

#[derive(Parser)]
struct Opt {
    #[clap(long, env = "RUST_LOG", default_value = "debug")]
    log_level: String,
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&opt.log_level))
        .init();

    let store: SharedStore = Arc::new(Mutex::new(Store::default()));

    let app = Router::new()
        .route("/mailbox/{addr}", post(post_mailbox).get(get_mailbox))
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::info!("relay listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
