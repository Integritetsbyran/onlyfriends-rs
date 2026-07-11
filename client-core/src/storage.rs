const SCHEMA_SQL: &'static str = "
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

CREATE TABLE IF NOT EXISTS seen_posts (
    id BLOB PRIMARY KEY,       -- Post.id, dedupe key
    author BLOB NOT NULL,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mailbox_cursors (
    friend_sign_pub BLOB NOT NULL,
    direction INTEGER NOT NULL,
    epoch INTEGER NOT NULL,
    last_index INTEGER NOT NULL,
    PRIMARY KEY (friend_sign_pub, direction, epoch)
);
";

pub struct Storage {
    conn: rusqlite::Connection,
}

impl Storage {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(SCHEMA_SQL)?; // the CREATE TABLE block above, as a const &str
        Ok(Storage { conn })
    }

    pub fn save_identity(&self, id: &keystone::Identity) -> rusqlite::Result<usize> {
        self.conn.execute(
            "INSERT OR REPLACE INTO identity (id, master_seed) VALUES (0, ?1)",
            [id.master_seed],
        )
    }

    pub fn load_identity(&self) -> Option<keystone::Identity> {
        match self
            .conn
            .query_row("SELECT master_seed FROM identity WHERE id = 0", [], |r| {
                r.get(0)
            }) {
            Ok(seed) => Some(keystone::Identity::from_seed(seed)),
            Err(_) => None
        }
    }

    pub fn save_friend(&self, f: &keystone::Friend) -> rusqlite::Result<usize> {
        self.conn.execute(
            "INSERT INTO friends (sign_pub, dh_pub, nickname, pairwise_root) VALUES (?1, ?2, ?3, ?4)", 
            (&f.public.sign_pub, &f.public.dh_pub, &f.nickname, &f.pairwise_root))
    }

    pub fn load_friends(&self) -> rusqlite::Result<Vec<keystone::Friend>> {
        let mut statment = self
            .conn
            .prepare("SELECT sign_pub, dh_pub, nickname, pairwise_root FROM friends")?;

        let rows = statment.query_map([], |row| {
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

        rows.collect()
    }

    pub fn get_cursor(&self, friend_sign_pub: &[u8; 32], direction: u8, epoch: u64) -> usize {
        self.conn
            .query_row(
                "SELECT last_index FROM mailbox_cursors
             WHERE friend_sign_pub = ?1 AND direction = ?2 AND epoch = ?3",
                (&friend_sign_pub[..], direction, epoch as i64),
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n as usize)
            .unwrap_or(0)
    }

    pub fn set_cursor(
        &self,
        friend_sign_pub: &[u8; 32],
        direction: u8,
        epoch: u64,
        index: usize,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO mailbox_cursors (friend_sign_pub, direction, epoch, last_index)
         VALUES (?1, ?2, ?3, ?4)",
            (&friend_sign_pub[..], direction, epoch as i64, index as i64),
        )?;
        Ok(())
    }
}
