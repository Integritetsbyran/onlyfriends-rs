use std::sync::{Mutex, MutexGuard};

use keystone::{identity::SigningPublicKey, post::PostId};
use storage_common::{
    storage::{Storage, StorageError, StorageResult},
    types::{stored_post::StoredPost, stored_response::StoredResponse},
};

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
pub enum SqliteStorageError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex error: {0}")]
    MutexError(String),
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
}

impl From<SqliteStorageError> for StorageError {
    fn from(err: SqliteStorageError) -> Self {
        match err {
            SqliteStorageError::SqliteError(e) => StorageError::QueryError(e.to_string()),
            SqliteStorageError::MutexError(e) => StorageError::QueryError(e),
            SqliteStorageError::StorageError(e) => e,
        }
    }
}

pub struct SqliteStorage {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteStorage {
    pub fn open(path: &str) -> Result<Self, SqliteStorageError> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(SqliteStorage {
            conn: Mutex::new(conn),
        })
    }

    fn get_conn(&self) -> Result<MutexGuard<'_, rusqlite::Connection>, SqliteStorageError> {
        self.conn
            .lock()
            .map_err(|err| SqliteStorageError::MutexError(err.to_string()))
    }

    /*
     * Inner implementations of the DB methods.
     */

    fn save_identity_impl(&self, id: &keystone::Identity) -> Result<usize, SqliteStorageError> {
        let conn = self.get_conn()?;
        Ok(conn.execute(
            "INSERT OR REPLACE INTO identity (id, master_seed) VALUES (0, ?1)",
            [id.master_seed],
        )?)
    }

    fn load_identity_impl(&self) -> Result<Option<keystone::Identity>, SqliteStorageError> {
        let conn = self.get_conn()?;
        match conn.query_row("SELECT master_seed FROM identity WHERE id = 0", [], |r| {
            r.get(0)
        }) {
            Ok(seed) => Ok(Some(keystone::Identity::from_seed(seed))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save_friend_impl(&self, f: &keystone::Friend) -> Result<(), SqliteStorageError> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO friends (sign_pub, dh_pub, nickname, pairwise_root) VALUES (?1, ?2, ?3, ?4)", 
            (&f.public.sign_pub, &f.public.dh_pub, &f.nickname, &f.pairwise_root))?;
        Ok(())
    }

    fn load_friends_impl(&self) -> Result<Vec<keystone::Friend>, SqliteStorageError> {
        let conn = self.get_conn()?;
        let mut stmt =
            conn.prepare("SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends")?;
        let rows = stmt.query_map([], |row| {
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
        })?;

        let friends: Result<Vec<_>, _> = rows.collect();
        Ok(friends?)
    }

    fn load_friend_by_sign_pub_impl(
        &self,
        sign_pub: &SigningPublicKey,
    ) -> Result<Option<keystone::Friend>, SqliteStorageError> {
        let conn = self.get_conn()?;
        let result = conn.query_row(
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
            Err(e) => Err(e.into()),
        }
    }

    fn save_profile_impl(&self, p: &keystone::Profile) -> Result<(), SqliteStorageError> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO profiles (owner, display_name, bio, version, sig)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (&p.owner[..], &p.display_name, &p.bio, p.version, &p.sig),
        )?;
        Ok(())
    }

    fn load_profile_impl(
        &self,
        owner_sign_pub: &SigningPublicKey,
    ) -> Result<Option<keystone::Profile>, SqliteStorageError> {
        let conn = self.get_conn()?;
        let result = conn.query_row(
            "SELECT owner, display_name, bio, version, sig FROM profiles WHERE owner = ?1",
            [&owner_sign_pub[..]],
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
            Err(e) => Err(e.into()),
        }
    }

    fn save_post_impl(
        &self,
        post: &keystone::Post,
        body: &str,
    ) -> Result<bool, SqliteStorageError> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "INSERT OR IGNORE INTO posts (id, author, body, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            (&post.id[..], &post.author[..], body, post.created_at as i64),
        )?;
        Ok(rows > 0)
    }

    fn load_posts_impl(&self) -> Result<Vec<StoredPost>, SqliteStorageError> {
        let conn = self.get_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, author, body, created_at FROM posts ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |r| {
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
        })?;
        let rows = rows.collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    fn save_response_impl(
        &self,
        post_id: &PostId,
        response: &StoredResponse,
    ) -> Result<bool, SqliteStorageError> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "INSERT OR IGNORE INTO responses (post_id, author, kind, content)
             VALUES (?1, ?2, ?3, ?4)",
            (
                &post_id[..],
                &response.author[..],
                u8::from(&response.kind),
                &response.content,
            ),
        )?;
        Ok(rows > 0)
    }

    fn load_responses_for_impl(
        &self,
        post_id: &PostId,
    ) -> Result<Vec<StoredResponse>, SqliteStorageError> {
        let conn = self.get_conn()?;
        let mut stmt =
            conn.prepare("SELECT author, kind, content FROM responses WHERE post_id = ?1")?;

        let rows = stmt.query_map([post_id], |r| {
            Ok(StoredResponse {
                author: r.get(0)?,
                kind: r.get::<_, u8>(1)?.try_into().map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Integer,
                        format!("invalid response kind: {}", err).into(),
                    )
                })?,
                content: r.get(2)?,
            })
        })?;
        let rows = rows.collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    fn get_cursor_impl(
        &self,
        friend_sign_pub: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> Result<usize, SqliteStorageError> {
        let conn = self.get_conn()?;
        let result = conn.query_row(
            "SELECT last_index FROM mailbox_cursors
             WHERE friend_sign_pub = ?1 AND direction = ?2 AND epoch = ?3",
            (&friend_sign_pub[..], direction, epoch as i64),
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(index) => Ok(index as usize),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    fn set_cursor_impl(
        &self,
        friend_sign_pub: &SigningPublicKey,
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> Result<(), SqliteStorageError> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO mailbox_cursors (friend_sign_pub, direction, epoch, last_index)
             VALUES (?1, ?2, ?3, ?4)",
            (&friend_sign_pub[..], direction, epoch as i64, index as i64),
        )?;
        Ok(())
    }
}

impl Storage for SqliteStorage {
    fn save_identity(&self, id: &keystone::Identity) -> StorageResult<usize> {
        Ok(self.save_identity_impl(id)?)
    }

    fn load_identity(&self) -> StorageResult<Option<keystone::Identity>> {
        Ok(self.load_identity_impl()?)
    }

    fn save_friend(&self, f: &keystone::Friend) -> StorageResult<()> {
        self.save_friend_impl(f)?;
        Ok(())
    }

    fn load_friends(&self) -> StorageResult<Vec<keystone::Friend>> {
        Ok(self.load_friends_impl()?)
    }

    fn load_friend_by_sign_pub(
        &self,
        sign_pub: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Friend>> {
        Ok(self.load_friend_by_sign_pub_impl(sign_pub)?)
    }

    fn save_profile(&self, p: &keystone::Profile) -> StorageResult<()> {
        self.save_profile_impl(p)?;
        Ok(())
    }

    fn load_profile(
        &self,
        owner_sign_pub: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Profile>> {
        Ok(self.load_profile_impl(owner_sign_pub)?)
    }

    fn save_post(&self, post: &keystone::Post, body: &str) -> StorageResult<bool> {
        Ok(self.save_post_impl(post, body)?)
    }

    fn load_posts(&self) -> StorageResult<Vec<StoredPost>> {
        Ok(self.load_posts_impl()?)
    }

    fn save_response(&self, post_id: &PostId, response: &StoredResponse) -> StorageResult<bool> {
        Ok(self.save_response_impl(post_id, response)?)
    }

    fn load_responses_for(&self, post_id: &PostId) -> StorageResult<Vec<StoredResponse>> {
        Ok(self.load_responses_for_impl(post_id)?)
    }

    fn get_cursor(
        &self,
        friend_sign_pub: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize> {
        Ok(self.get_cursor_impl(friend_sign_pub, direction, epoch)?)
    }

    fn set_cursor(
        &self,
        friend_sign_pub: &SigningPublicKey,
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> StorageResult<()> {
        Ok(self.set_cursor_impl(friend_sign_pub, direction, epoch, index)?)
    }
}
