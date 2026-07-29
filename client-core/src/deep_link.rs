//! Deep link parsing/building for the "add friend" flow.
//!
//! Two link forms are supported:
//! - **Universal / App Link** (preferred): `https://of.integritetsbyran.org/add/<friend-key>`.
//!   If the app is installed, iOS/Android hand this straight to the app.
//!
//! - **Custom scheme**: `onlyfriends://add/<friend-key>`. Used for
//!   OS-level protocol registration on desktop (Linux/macOS/Windows), and
//!   accepted as a fallback when parsing incoming links on any platform.

/// Custom URL scheme used for desktop OS-level protocol registration.
pub const DEEP_LINK_SCHEME: &str = "onlyfriends";

/// Placeholder universal-link host.
pub const DEEP_LINK_HOST: &str = "of.integritetsbyran.org";

/// Path segment used for "add friend" links, e.g. `/add/<code>`.
pub const ADD_FRIEND_PATH: &str = "add";

/// Build the link to embed in the "add me" QR code / share sheet.
pub fn build_add_friend_link(friend_key: &str) -> String {
    format!("https://{DEEP_LINK_HOST}/{ADD_FRIEND_PATH}/{friend_key}")
}

/// Build the custom-scheme form of the same link. Used when registering
/// `onlyfriends://` as an OS-level protocol handler on desktop.
pub fn build_add_friend_scheme_link(hex_public_key: &str) -> String {
    format!("{DEEP_LINK_SCHEME}://{ADD_FRIEND_PATH}/{hex_public_key}")
}

/// Try to extract the hex-encoded public key out of any supported
/// "add friend" deep link form. Returns `None` if `input` doesn't look like
/// one of our links.
///
/// Accepts, in order:
/// - `onlyfriends://add/<hex>`
/// - `https://<DEEP_LINK_HOST>/add/<hex>`
/// - `http://<DEEP_LINK_HOST>/add/<hex>` (dev/test setups)
pub fn parse_add_friend_link(input: &str) -> Option<String> {
    let input = input.trim();

    let rest = input
        .strip_prefix(DEEP_LINK_SCHEME)
        .and_then(|r| r.strip_prefix("://"))
        .or_else(|| input.strip_prefix("https://")?.strip_prefix(DEEP_LINK_HOST))
        .or_else(|| input.strip_prefix("http://")?.strip_prefix(DEEP_LINK_HOST))?;
    let rest = rest.trim_start_matches('/');

    let code = rest
        .strip_prefix(ADD_FRIEND_PATH)
        .and_then(|r| r.strip_prefix('/'))
        .unwrap_or(rest);
    let code = code.trim_matches('/');

    if code.is_empty() || !is_hex(code) {
        return None;
    }

    Some(code.to_string())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.len().is_multiple_of(2) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Scan process arguments (e.g. `std::env::args()`) for one that looks like
/// an "add friend" deep link — passed by the OS when launching the app via a
/// registered `onlyfriends://` custom scheme or universal link — and return
/// the decoded hex code if found.
///
/// This only covers the "cold start via link" case. Handling a link
/// delivered while the app is already running requires a native
/// single-instance/IPC mechanism (desktop) or `onNewIntent`/`openURL`
/// bridging (Android/iOS) that isn't wired up yet.
pub fn find_add_friend_arg<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .find_map(|arg| parse_add_friend_link(arg.as_ref()))
}
