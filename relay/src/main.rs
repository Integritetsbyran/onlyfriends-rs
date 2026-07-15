use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use base64::engine::general_purpose::STANDARD;
use clap::Parser;
use keystone::{
    SealedBox,
    envelope::{Envelope, Letter, LetterKey, UserId},
};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use crate::deserialize::Postcard;

mod deserialize;

#[derive(Serialize)]
struct MailboxEntry {
    letter: Arc<Letter>,
    key: SealedBox<LetterKey>,
}

#[derive(Default)]
struct Store {
    mailboxes: HashMap<UserId, VecDeque<MailboxEntry>>,
}

type SharedStore = Arc<Mutex<Store>>;

#[derive(Deserialize)]
struct PostItemRequest {
    item_b64: String,
}

async fn post_mailbox(
    State(store): State<SharedStore>,
    Postcard(envelope): Postcard<Envelope>,
) -> Result<(), axum::http::StatusCode> {
    let mut store = store.lock().unwrap();

    let Envelope { letter, recipients } = envelope;
    let letter = Arc::new(letter);

    for recipient in recipients {
        let entry = store.mailboxes.entry(recipient.id).or_default();
        entry.push_back(MailboxEntry {
            letter: Arc::clone(&letter),
            key: recipient.key,
        });
    }
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
) -> Result<Postcard<MailboxEntry>, StatusCode> {
    use base64::Engine;

    let addr: UserId = STANDARD
        .decode(addr)
        .ok()
        .and_then(|addr| addr.try_into().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut store = store.lock().unwrap();
    let entry: MailboxEntry = store
        .mailboxes
        .get_mut(&addr)
        .and_then(|v| v.pop_front())
        .ok_or(StatusCode::NO_CONTENT)?;

    Ok(Postcard(entry))
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
        .route("/mailbox", post(post_mailbox))
        .route("/mailbox/{addr}", get(get_mailbox))
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::info!("relay listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
