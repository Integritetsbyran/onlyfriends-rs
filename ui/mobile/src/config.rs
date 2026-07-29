use std::path::PathBuf;

/// Path to the SQLite database file.
///
/// Mobile app sandboxes don't allow writing to the process's working
/// directory, so each platform needs its own writable, persistent
/// directory. Desktop (used by `dx serve` during local development) keeps
/// the simple relative path.
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

/// Android has no `HOME`-style shortcut; the app's private storage
/// directory has to be fetched from the JVM via `Context.getFilesDir()`.
#[cfg(target_os = "android")]
fn app_data_dir() -> PathBuf {
    let ctx = ndk_context::android_context();
    // SAFETY: `ctx.vm()`/`ctx.context()` are valid JVM/Activity pointers
    // supplied by `ndk-context`, which is initialized by the Android
    // activity glue before any app code runs.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.expect("attach to JavaVM");
    let mut env = vm.attach_current_thread().expect("attach current thread");
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };

    let files_dir = env
        .call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[])
        .and_then(|v| v.l())
        .expect("Context.getFilesDir()");
    let path = env
        .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .and_then(|v| v.l())
        .expect("File.getAbsolutePath()");
    let path: String = env
        .get_string(&jni::objects::JString::from(path))
        .expect("getAbsolutePath() should return a string")
        .into();

    PathBuf::from(path)
}

/// Desktop (and `dx serve` running the mobile crate on the host during
/// development) — no sandbox restrictions, keep it simple.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn app_data_dir() -> PathBuf {
    PathBuf::from(".")
}
