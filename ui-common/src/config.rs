/// Path to the SQLite database file.
pub fn db_path() -> String {
    "./onlyfriends.db".to_string()
}

/// Path to the relay-URL config file (one line of plain text).
fn relay_path() -> &'static str {
    "./onlyfriends-relay.txt"
}

/// Read the persisted relay URL, if any.
pub fn load_relay_url() -> Option<String> {
    std::fs::read_to_string(relay_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Persist the relay URL to disk.
pub fn save_relay_url(url: &str) -> std::io::Result<()> {
    std::fs::write(relay_path(), url)
}
