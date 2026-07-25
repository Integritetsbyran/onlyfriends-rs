use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::post,
};
use clap::Parser;
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use crate::postcard::{Postcard, PostcardRaw};

pub mod postcard;

#[derive(Default)]
struct Store {
    mailboxes: HashMap<String, Vec<Vec<u8>>>,
}

type SharedStore = Arc<Mutex<Store>>;

async fn post_mailbox(
    State(store): State<SharedStore>,
    Path(addr): Path<String>,
    PostcardRaw(bytes): PostcardRaw,
) -> Result<(), StatusCode> {
    let mut store = store.lock().unwrap();
    store.mailboxes.entry(addr).or_default().push(bytes.into());
    Ok(())
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
) -> Postcard<Vec<Vec<u8>>> {
    let store = store.lock().unwrap();
    let items = store.mailboxes.get(&addr).map(Vec::as_slice).unwrap_or(&[]);
    let items = items.get(q.after..).unwrap_or(&[]);
    Postcard(items.to_vec())
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
