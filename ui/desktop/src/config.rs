use std::error::Error;
use std::path::PathBuf;

/// Path to the SQLite database file.
#[cfg(target_os = "linux")]
pub fn db_path() -> Result<PathBuf, Box<dyn Error>> {
    let xdg = xdg::BaseDirectories::new();
    let dir = xdg.create_data_directory("org.integritetsbyran.OnlyFriends")?;
    Ok(dir.join("onlyfriends.sqlite"))
}

/// Path to the SQLite database file.
#[cfg(not(target_os = "linux"))]
pub fn db_path() -> Result<PathBuf, Box<dyn Error>> {
    // TODO
    Ok("./onlyfriends.db".into())
}
