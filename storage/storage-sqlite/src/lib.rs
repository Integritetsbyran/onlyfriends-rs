use std::{path::Path, str::FromStr};

use keystone::{
    Identity,
    envelope::{Letter, LetterId},
    identity::{MasterSeed, SigningPublicKey},
    media::Media,
    post::PostContent,
};
use mime::Mime;
use storage_common::{
    storage::{Storage, StorageError, StorageResult},
    types::{relay_config::RelayConfig, stored_post::StoredPost, stored_response::StoredResponse},
};

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS identity (
    id INTEGER PRIMARY KEY CHECK (id = 0),  -- singleton row
    master_seed BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS relay_config (
    id INTEGER PRIMARY KEY CHECK (id = 0),  -- singleton row
    url TEXT NOT NULL
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

CREATE TABLE IF NOT EXISTS post_media (
    id INTEGER PRIMARY KEY,
    post_id BLOB NOT NULL REFERENCES posts(id),
    mime TEXT NOT NULL,
    bytes BLOB NOT NULL
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
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SqliteStorageError> {
        let conn = rusqlite::Connection::open(path.as_ref())?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(SqliteStorage { conn })
    }

    /*
     * Inner implementations of the DB methods.
     */

    fn save_identity_inner(&mut self, id: &keystone::Identity) -> Result<(), SqliteStorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO identity (id, master_seed) VALUES (0, ?1)",
            [id.master_seed.to_bytes()],
        )?;
        Ok(())
    }

    fn load_identity_inner(&mut self) -> Result<Option<keystone::Identity>, SqliteStorageError> {
        match self
            .conn
            .query_row("SELECT master_seed FROM identity WHERE id = 0", [], |r| {
                let bytes: [u8; 32] = r.get(0)?;
                Ok(MasterSeed::from(bytes))
            }) {
            Ok(seed) => Ok(Some(Identity::from_seed(seed))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save_relay_config_inner(&self, config: &RelayConfig) -> Result<(), SqliteStorageError> {
        let RelayConfig { url } = config;
        self.conn.execute(
            "INSERT OR REPLACE INTO relay_config (id, url) VALUES (0, ?1)",
            [url],
        )?;
        Ok(())
    }

    fn load_relay_config_inner(&self) -> Result<Option<RelayConfig>, SqliteStorageError> {
        self.conn
            .query_row("SELECT url FROM relay_config WHERE id = 0", [], |r| {
                let url: String = r.get(0)?;
                Ok(Some(RelayConfig { url }))
            })
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e.into()),
            })
    }

    fn save_friend_inner(&mut self, f: &keystone::Friend) -> Result<(), SqliteStorageError> {
        self.conn.execute(
            "INSERT INTO friends (sign_pub, dh_pub, nickname, pairwise_root) VALUES (?1, ?2, ?3, ?4)", 
            (&f.public.sign_pub.to_bytes(), &f.public.dh_pub.to_bytes(), &f.nickname, &f.pairwise_root))?;
        Ok(())
    }

    fn load_friends_inner(&mut self) -> Result<Vec<keystone::Friend>, SqliteStorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends")?;
        let rows = stmt.query_map([], |row| {
            let sign_pub: [u8; 32] = row.get(0)?;
            let dh_pub: [u8; 32] = row.get(1)?;
            let nickname: String = row.get(2)?;
            let pairwise_root: [u8; 32] = row.get(3)?;

            Ok(keystone::Friend {
                public: keystone::PublicIdentity {
                    sign_pub: sign_pub.into(),
                    dh_pub: dh_pub.into(),
                },
                nickname,
                pairwise_root,
            })
        })?;

        let friends: Result<Vec<_>, _> = rows.collect();
        Ok(friends?)
    }

    fn load_friend_by_sign_pub_inner(
        &mut self,
        sign_pub: &SigningPublicKey,
    ) -> Result<Option<keystone::Friend>, SqliteStorageError> {
        let result = self.conn.query_row(
            "SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends WHERE sign_pub = ?1",
            [sign_pub.to_bytes()],
            |r| {
                let sign_pub: [u8; 32] = r.get(0)?;
                let dh_pub: [u8; 32] = r.get(1)?;
                let nickname: String = r.get(2)?;
                let pairwise_root: [u8; 32] = r.get(3)?;

                Ok(keystone::Friend {
                    public: keystone::PublicIdentity {
                        sign_pub: sign_pub.into(),
                        dh_pub: dh_pub.into(),
                    },
                    nickname,
                    pairwise_root,
                })
            },
        );

        match result {
            Ok(friend) => Ok(Some(friend)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save_profile_inner(&mut self, p: &keystone::Profile) -> Result<(), SqliteStorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO profiles (owner, display_name, bio, version, sig)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &p.owner.to_byte_slice(),
                &p.display_name,
                &p.bio,
                p.version,
                &p.sig.to_bytes(),
            ),
        )?;
        Ok(())
    }

    fn load_profile_inner(
        &mut self,
        owner: &SigningPublicKey,
    ) -> Result<Option<keystone::Profile>, SqliteStorageError> {
        let result = self.conn.query_row(
            "SELECT owner, display_name, bio, version, sig FROM profiles WHERE owner = ?1",
            [owner.to_byte_slice()],
            |r| {
                let bytes: [u8; 32] = r.get(0)?;
                Ok(keystone::Profile {
                    owner: SigningPublicKey::from(bytes),
                    display_name: r.get(1)?,
                    bio: r.get(2)?,
                    version: r.get(3)?,
                    sig: r.get::<_, [u8; 64]>(4)?.into(),
                })
            },
        );

        match result {
            Ok(profile) => Ok(Some(profile)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save_post_inner(
        &mut self,
        encrypted: &Letter,
        post: &PostContent,
    ) -> Result<bool, SqliteStorageError> {
        let transaction = self.conn.transaction()?;

        let rows = transaction.execute(
            "INSERT OR IGNORE INTO posts (id, author, body, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            (
                encrypted.id.to_byte_slice(),
                encrypted.author.to_byte_slice(),
                &post.body,
                encrypted.created_at as i64,
            ),
        )?;
        let did_insert = rows > 0;

        if did_insert {
            for media in &post.media {
                transaction.execute(
                    "INSERT INTO post_media (post_id, mime, bytes)
                     VALUES (?1, ?2, ?3)",
                    (
                        encrypted.id.to_byte_slice(),
                        media.mime.as_ref(),
                        &media.bytes,
                    ),
                )?;
            }
        }

        transaction.commit()?;

        Ok(did_insert)
    }

    fn load_posts_inner(&mut self) -> Result<Vec<StoredPost>, SqliteStorageError> {
        let transaction = self.conn.transaction()?;

        let mut select_posts = transaction
            .prepare("SELECT id, author, body, created_at FROM posts ORDER BY created_at DESC")?;

        let mut select_media =
            transaction.prepare("SELECT mime, bytes FROM post_media WHERE post_id = ?1")?;

        let rows = select_posts.query_map([], |r| {
            let id: [u8; 16] = r.get(0)?;
            let author: [u8; 32] = r.get(1)?;

            let post = StoredPost {
                id: id.into(),
                author: author.into(),
                body: r.get(2)?,
                created_at: r.get::<_, i64>(3)? as u64,
                media: vec![],
            };

            Ok(post)
        })?;
        let mut posts: Vec<StoredPost> = rows.collect::<Result<_, _>>()?;

        for post in &mut posts {
            let media_rows = select_media.query_map([post.id.to_byte_slice()], |r| {
                let mime: String = r.get(0)?;
                Ok(Media {
                    mime: Mime::from_str(&mime).unwrap(),
                    bytes: r.get(1)?,
                })
            })?;

            for media in media_rows {
                post.media.push(media?);
            }
        }

        Ok(posts)
    }

    fn save_response_inner(
        &mut self,
        letter_id: &LetterId,
        response: &StoredResponse,
    ) -> Result<bool, SqliteStorageError> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO responses (post_id, author, kind, content)
             VALUES (?1, ?2, ?3, ?4)",
            (
                letter_id.to_byte_slice(),
                response.author.to_byte_slice(),
                u8::from(&response.kind),
                &response.content,
            ),
        )?;
        Ok(rows > 0)
    }

    fn load_responses_for_inner(
        &mut self,
        letter_id: &LetterId,
    ) -> Result<Vec<StoredResponse>, SqliteStorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT author, kind, content FROM responses WHERE post_id = ?1")?;

        let rows = stmt.query_map([letter_id.to_byte_slice()], |r| {
            let author: [u8; 32] = r.get(0)?;
            Ok(StoredResponse {
                author: author.into(),
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

    fn get_cursor_inner(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> Result<usize, SqliteStorageError> {
        let result = self.conn.query_row(
            "SELECT last_index FROM mailbox_cursors
             WHERE friend_sign_pub = ?1 AND direction = ?2 AND epoch = ?3",
            (friend.to_byte_slice(), direction, epoch as i64),
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(index) => Ok(index as usize),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    fn set_cursor_inner(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> Result<(), SqliteStorageError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO mailbox_cursors (friend_sign_pub, direction, epoch, last_index)
             VALUES (?1, ?2, ?3, ?4)",
            (
                friend.to_byte_slice(),
                direction,
                epoch as i64,
                index as i64,
            ),
        )?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Storage for SqliteStorage {
    async fn save_identity(&mut self, id: &keystone::Identity) -> StorageResult<()> {
        self.save_identity_inner(id)?;
        Ok(())
    }

    async fn load_identity(&mut self) -> StorageResult<Option<keystone::Identity>> {
        Ok(self.load_identity_inner()?)
    }

    async fn save_relay_config(&mut self, config: &RelayConfig) -> StorageResult<()> {
        self.save_relay_config_inner(config)?;
        Ok(())
    }

    async fn load_relay_config(&mut self) -> StorageResult<Option<RelayConfig>> {
        Ok(self.load_relay_config_inner()?)
    }

    async fn save_friend(&mut self, f: &keystone::Friend) -> StorageResult<()> {
        self.save_friend_inner(f)?;
        Ok(())
    }

    async fn load_friends(&mut self) -> StorageResult<Vec<keystone::Friend>> {
        Ok(self.load_friends_inner()?)
    }

    async fn load_friend_by_sign_pub(
        &mut self,
        friend: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Friend>> {
        Ok(self.load_friend_by_sign_pub_inner(friend)?)
    }

    async fn save_profile(&mut self, p: &keystone::Profile) -> StorageResult<()> {
        self.save_profile_inner(p)?;
        Ok(())
    }

    async fn load_profile(
        &mut self,
        owner: &SigningPublicKey,
    ) -> StorageResult<Option<keystone::Profile>> {
        Ok(self.load_profile_inner(owner)?)
    }

    async fn save_post(
        &mut self,
        encrypted: &keystone::Letter,
        post: &PostContent,
    ) -> StorageResult<bool> {
        Ok(self.save_post_inner(encrypted, post)?)
    }

    async fn load_posts(&mut self) -> StorageResult<Vec<StoredPost>> {
        Ok(self.load_posts_inner()?)
    }

    async fn save_response(
        &mut self,
        letter_id: &LetterId,
        response: &StoredResponse,
    ) -> StorageResult<bool> {
        Ok(self.save_response_inner(letter_id, response)?)
    }

    async fn load_responses_for(
        &mut self,
        letter_id: &LetterId,
    ) -> StorageResult<Vec<StoredResponse>> {
        Ok(self.load_responses_for_inner(letter_id)?)
    }

    async fn get_cursor(
        &mut self,
        friend: &SigningPublicKey,
        direction: u8,
        epoch: u64,
    ) -> StorageResult<usize> {
        Ok(self.get_cursor_inner(friend, direction, epoch)?)
    }

    async fn set_cursor(
        &mut self,
        friend: SigningPublicKey,
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> StorageResult<()> {
        Ok(self.set_cursor_inner(&friend, direction, epoch, index)?)
    }
}
