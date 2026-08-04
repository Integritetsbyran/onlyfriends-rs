use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::post,
};
use clap::Parser;
use serde::Deserialize;
use tower_http::{
    cors::{self, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use self::postcard::{Postcard, PostcardRaw};

mod cleanup;
mod postcard;

struct MailboxEntry {
    pub uploaded_at: Instant,
    pub blob: Vec<u8>,
}

#[derive(Default)]
struct Store {
    mailboxes: HashMap<String, Vec<MailboxEntry>>,
}

type SharedStore = Arc<Mutex<Store>>;

async fn post_mailbox(
    State(store): State<SharedStore>,
    Path(addr): Path<String>,
    PostcardRaw(bytes): PostcardRaw,
) -> Result<(), StatusCode> {
    let mut store = store.lock().unwrap();
    let entry = MailboxEntry {
        uploaded_at: Instant::now(),
        blob: bytes.into(),
    };
    store.mailboxes.entry(addr).or_default().push(entry);
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
    let entries = store.mailboxes.get(&addr).map(Vec::as_slice).unwrap_or(&[]);
    let entries = entries.get(q.after..).unwrap_or(&[]);
    let blobs = entries.iter().map(|entry| entry.blob.clone()).collect();
    Postcard(blobs)
}

#[derive(Parser)]
struct Opt {
    /// IP and port to bind to.
    #[clap(long, env = "OF_RELAY_BIND", default_value = "127.0.0.1:3000")]
    bind: SocketAddr,

    #[clap(long, env = "RUST_LOG", default_value = "debug")]
    log_level: String,

    /// The maximum age of mailbox entries in seconds.
    #[clap(long, env = "OF_MAILBOX_MAX_AGE", default_value = "604800")]
    mailbox_max_age: u64,
}

#[tokio::main]
pub async fn run() {
    let opt = Opt::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&opt.log_level))
        .init();

    let store: SharedStore = Arc::new(Mutex::new(Store::default()));

    cleanup::start_task(&store, &opt);

    let cors = CorsLayer::new()
        .allow_methods(cors::Any)
        .allow_origin(cors::Any)
        .allow_headers(cors::Any);

    let app = Router::new()
        .route("/mailbox/{addr}", post(post_mailbox).get(get_mailbox))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(&opt.bind).await.unwrap();
    tracing::info!("relay listening on http://{}", opt.bind);
    axum::serve(listener, app).await.unwrap();
}
