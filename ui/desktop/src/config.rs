use std::path::PathBuf;

/// Path to the SQLite database file.
///
/// Uses the OS-appropriate per-user app data directory (e.g.
/// `~/.local/share`, `~/Library/Application Support`, `%APPDATA%`) so the
/// app doesn't depend on being launched from a writable working directory —
/// falls back to the current directory if that can't be determined.
pub fn db_path() -> String {
    let dir = dirs::data_dir()
        .map(|d| d.join("onlyfriends"))
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir).ok();
    dir.join("onlyfriends.db").display().to_string()
}
