use std::path::PathBuf;

/// Path to the SQLite database file.
pub fn db_path() -> String {
    app_data_dir().join("onlyfriends.db").display().to_string()
}

/// iOS sets `HOME` to the app's sandbox container before the process
/// starts; `Library/Application Support` is Apple's recommended location
/// for persistent app data.
#[cfg(target_os = "ios")]
fn app_data_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME must be set on iOS");
    let dir = PathBuf::from(home).join("Library/Application Support");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Android has no `HOME`-style shortcut; use the native activity's
/// private app data directory (equivalent to `Context.getFilesDir()`).
#[cfg(target_os = "android")]
fn app_data_dir() -> PathBuf {
    let dir = dioxus_native::current_android_app()
        .internal_data_path()
        .expect("Android internal_data_path should be available");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Desktop (and `dx serve` running the mobile crate on the host during
/// development) — no sandbox restrictions, keep it simple.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn app_data_dir() -> PathBuf {
    PathBuf::from(".")
}
