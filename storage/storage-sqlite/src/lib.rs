use storage_common::{
    storage::{Storage, StorageError, StorageResult},
    types::{stored_post::StoredPost, stored_response::StoredResponse},
};

// TODO: Ayoh ya matey, welcome to crime city. Here be no laws that have not been broken.
// This is a crime-scene of a file. It is a crime to read this file without a warrant.
// If you are reading this, you are hereby warned that you are committing a crime, GL & HF.

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

#[derive(Debug, thiserror::Error)]
enum SqliteStorageError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
}

impl Into<StorageError> for SqliteStorageError {
    fn into(self) -> StorageError {
        match self {
            SqliteStorageError::SqliteError(err) => StorageError::QueryError(err.to_string()),
        }
    }
}

// Converts a compatible error into SqliteStorageError and then into StorageError.
macro_rules! sqlite_err {
    ($($arg:tt)*) => {
        SqliteStorageError::SqliteError($($arg)*).into()
    };
}

// Effectively a hacky `?` operator but that does a double-convertion of compatible errors utilizing `sqlite_err!()` macro.
macro_rules! sqlite_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(sqlite_err!(e)),
        }
    };
}

pub struct SqliteStorage {
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    pub fn open(path: &str) -> Result<Self, StorageError> {
        let conn = sqlite_result!(rusqlite::Connection::open(path));
        sqlite_result!(conn.execute_batch(SCHEMA_SQL));
        Ok(SqliteStorage { conn })
    }
}

impl Storage for SqliteStorage {
    fn save_identity(&self, id: &keystone::Identity) -> StorageResult<usize> {
        Ok(sqlite_result!(self.conn.execute(
            "INSERT OR REPLACE INTO identity (id, master_seed) VALUES (0, ?1)",
            [id.master_seed],
        )))
    }

    fn load_identity(&self) -> StorageResult<Option<keystone::Identity>> {
        match self
            .conn
            .query_row("SELECT master_seed FROM identity WHERE id = 0", [], |r| {
                r.get(0)
            }) {
            Ok(seed) => Ok(Some(keystone::Identity::from_seed(seed))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(sqlite_err!(e)),
        }
    }

    fn save_friend(&self, f: &keystone::Friend) -> StorageResult<usize> {
        Ok(sqlite_result!(self.conn.execute(
            "INSERT INTO friends (sign_pub, dh_pub, nickname, pairwise_root) VALUES (?1, ?2, ?3, ?4)", 
            (&f.public.sign_pub, &f.public.dh_pub, &f.nickname, &f.pairwise_root))
        ))
    }

    fn load_friends(&self) -> StorageResult<Vec<keystone::Friend>> {
        let mut statment = sqlite_result!(
            self.conn
                .prepare("SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends")
        );

        let rows = sqlite_result!(statment.query_map([], |row| {
            let sign_pub: Vec<u8> = row.get(0)?;
            let dh_pub: Vec<u8> = row.get(1)?;
            let nickname: String = row.get(2)?;
            let pairwise_root: Vec<u8> = row.get(3)?;

            Ok(keystone::Friend {
                public: keystone::PublicIdentity {
                    sign_pub: sign_pub.try_into().unwrap(),
                    dh_pub: dh_pub.try_into().unwrap(),
                },
                nickname,
                pairwise_root: pairwise_root.try_into().unwrap(),
            })
        }));

        let rows = sqlite_result!(rows.collect());
        Ok(rows)
    }

    fn load_friend_by_sign_pub(
        &self,
        sign_pub: &[u8; 32],
    ) -> StorageResult<Option<keystone::Friend>> {
        // same query shape as load_friends, but WHERE sign_pub = ?1, query_row instead of query_map
        let result = self.conn.query_row(
            "SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends WHERE sign_pub = ?1",
            [sign_pub],
            |r| {
                let sign_pub: Vec<u8> = r.get(0)?;
                let dh_pub: Vec<u8> = r.get(1)?;
                let nickname: String = r.get(2)?;
                let pairwise_root: Vec<u8> = r.get(3)?;

                Ok(keystone::Friend {
                    public: keystone::PublicIdentity {
                        sign_pub: sign_pub.try_into().unwrap(),
                        dh_pub: dh_pub.try_into().unwrap(),
                    },
                    nickname,
                    pairwise_root: pairwise_root.try_into().unwrap(),
                })
            },
        );

        match result {
            Ok(friend) => Ok(Some(friend)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(sqlite_err!(e)),
        }
    }

    fn save_profile(&self, p: &keystone::Profile) -> StorageResult<usize> {
        Ok(sqlite_result!(self.conn.execute(
            "INSERT OR REPLACE INTO profiles (owner, display_name, bio, version, sig)
         VALUES (?1, ?2, ?3, ?4, ?5)",
            (&p.owner[..], &p.display_name, &p.bio, p.version, &p.sig),
        )))
    }

    fn load_profile(&self, owner: &[u8; 32]) -> StorageResult<Option<keystone::Profile>> {
        let result = self.conn.query_row(
            "SELECT owner, display_name, bio, version, sig FROM profiles WHERE owner = ?1",
            [&owner[..]],
            |r| {
                Ok(keystone::Profile {
                    owner: r.get(0)?,
                    display_name: r.get(1)?,
                    bio: r.get(2)?,
                    version: r.get(3)?,
                    sig: r.get(4)?,
                })
            },
        );

        match result {
            Ok(profile) => Ok(Some(profile)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(sqlite_err!(e)),
        }
    }

    fn save_post(&self, post: &keystone::Post, body: &str) -> StorageResult<bool> {
        let rows = sqlite_result!(self.conn.execute(
            "INSERT OR IGNORE INTO posts (id, author, body, created_at)
         VALUES (?1, ?2, ?3, ?4)",
            (&post.id[..], &post.author[..], body, post.created_at as i64),
        ));
        Ok(rows > 0)
    }

    fn load_posts(&self) -> StorageResult<Vec<StoredPost>> {
        let mut stmt = sqlite_result!(
            self.conn
                .prepare("SELECT id, author, body, created_at FROM posts ORDER BY created_at DESC")
        );
        let rows = sqlite_result!(stmt.query_map([], |r| {
            Ok(StoredPost {
                id: r.get::<_, Vec<u8>>(0)?.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        "bad id length".into(),
                    )
                })?,
                author: r.get::<_, Vec<u8>>(1)?.try_into().map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Blob,
                        "bad author length".into(),
                    )
                })?,
                body: r.get(2)?,
                created_at: r.get::<_, i64>(3)? as u64,
            })
        }));
        let rows = sqlite_result!(rows.collect());
        Ok(rows)
    }

    fn save_response(
        &self,
        post_id: &[u8; 16],
        author: &[u8; 32],
        kind: u8,
        content: &str,
    ) -> StorageResult<bool> {
        let rows = sqlite_result!(self.conn.execute(
            "INSERT OR IGNORE INTO responses (post_id, author, kind, content)
         VALUES (?1, ?2, ?3, ?4)",
            (&post_id[..], &author[..], kind, content),
        ));
        Ok(rows > 0)
    }

    fn load_responses_for(&self, post_id: &[u8; 16]) -> StorageResult<Vec<StoredResponse>> {
        let mut stmt = sqlite_result!(
            self.conn
                .prepare("SELECT author, kind, content FROM responses WHERE post_id = ?1")
        );
        let rows = sqlite_result!(stmt.query_map([post_id], |r| {
            Ok(StoredResponse {
                author: r.get(0)?,
                kind: r.get(1)?,
                content: r.get(2)?,
            })
        }));
        let rows = sqlite_result!(rows.collect());
        Ok(rows)
    }

    fn get_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize> {
        Ok(self
            .conn
            .query_row(
                "SELECT last_index FROM mailbox_cursors
             WHERE friend_sign_pub = ?1 AND direction = ?2 AND epoch = ?3",
                (&friend_sign_pub[..], direction, epoch as i64),
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n as usize)
            .unwrap_or(0))
    }

    fn set_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> StorageResult<()> {
        sqlite_result!(self.conn.execute(
            "INSERT OR REPLACE INTO mailbox_cursors (friend_sign_pub, direction, epoch, last_index)
         VALUES (?1, ?2, ?3, ?4)",
            (&friend_sign_pub[..], direction, epoch as i64, index as i64),
        ));
        Ok(())
    }
}
