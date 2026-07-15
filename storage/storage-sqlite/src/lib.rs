use storage_common::storage::Storage;

#[derive(Debug, thiserror::Error)]
pub enum SqliteStorageError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
}

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS identity (
    id INTEGER PRIMARY KEY CHECK (id = 0),  -- singleton row
    master_seed BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS friends (
    sign_pub BLOB PRIMARY KEY,
    dh_pub BLOB NOT NULL,
    nickname TEXT NOT NULL,
    pairwise_root BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS profiles (
    owner BLOB PRIMARY KEY,      -- sign_pub this profile belongs to
    display_name TEXT NOT NULL,
    bio TEXT NOT NULL,
    avatar_hash BLOB,            -- nullable; blob bytes stored separately (later)
    version INTEGER NOT NULL,
    sig BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS posts (
    id BLOB PRIMARY KEY,       -- Post.id, dedupe key
    author BLOB NOT NULL,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS responses (
    post_id BLOB NOT NULL,
    author BLOB NOT NULL,
    kind INTEGER NOT NULL,       -- 0 = reaction, 1 = comment
    content TEXT NOT NULL,       -- emoji string, or decrypted comment text
    PRIMARY KEY (post_id, author, kind)
);

CREATE TABLE IF NOT EXISTS mailbox_cursors (
    friend_sign_pub BLOB NOT NULL,
    direction INTEGER NOT NULL,
    epoch INTEGER NOT NULL,
    last_index INTEGER NOT NULL,
    PRIMARY KEY (friend_sign_pub, direction, epoch)
);
";

pub struct SqliteStorage {
    conn: rusqlite::Connection,
}

pub trait Storable {}

impl SqliteStorage {
    pub fn open(path: &str) -> Result<Self, SqliteStorageError> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(SCHEMA_SQL)?; // the create table block above, as a const &str
        Ok(Self { conn })
    }
}

impl<T: Storable> Storage<T> for SqliteStorage {
    type Error = SqliteStorageError;

    fn save(&mut self, obj: T) -> storage_common::error::StorageResult<()> {
        todo!()
    }
}
