//! Deep link parsing/building for the "add friend" flow.
//!
//! Two link forms are supported:
//! - **Universal / App Link** (preferred): `https://<DEEP_LINK_HOST>/add/<hex-public-key>`.
//!   If the app is installed, iOS/Android hand this straight to the app
//!   (Associated Domains / App Links). If it isn't installed, the OS falls
//!   back to opening it in the browser, where a download/landing page can
//!   live at that same URL.
//! - **Custom scheme**: `onlyfriends://add/<hex-public-key>`. Used for
//!   OS-level protocol registration on desktop (Linux/macOS/Windows), and
//!   accepted as a fallback when parsing incoming links on any platform.
//!   Unlike the universal link, this form has no automatic "app not
//!   installed" fallback on mobile.
//!
//! `DEEP_LINK_HOST` and the bundle identifier in `ui/mobile/Dioxus.toml`
//! are placeholders until real production values are chosen — update them
//! together, along with `well-known/apple-app-site-association` and
//! `well-known/assetlinks.json`.

/// Custom URL scheme used for desktop OS-level protocol registration.
pub const DEEP_LINK_SCHEME: &str = "onlyfriends";

/// Placeholder universal-link host. Replace with the real production
/// domain once one is registered — see `well-known/README.md`.
pub const DEEP_LINK_HOST: &str = "onlyfriends.app";

/// Path segment used for "add friend" links, e.g. `/add/<code>`.
pub const ADD_FRIEND_PATH: &str = "add";

/// Build the link to embed in the "add me" QR code / share sheet.
///
/// Uses the `https://` universal-link form so that scanning still leads
/// somewhere useful (a download page) even if the app isn't installed yet.
pub fn build_add_friend_link(hex_public_key: &str) -> String {
    format!("https://{DEEP_LINK_HOST}/{ADD_FRIEND_PATH}/{hex_public_key}")
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
/// - `onlyfriends://<hex>` (legacy form, pre-universal-links)
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

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "deadbeef00112233";

    #[test]
    fn builds_universal_link() {
        assert_eq!(
            build_add_friend_link(CODE),
            format!("https://{DEEP_LINK_HOST}/add/{CODE}")
        );
    }

    #[test]
    fn parses_universal_link() {
        let link = build_add_friend_link(CODE);
        assert_eq!(parse_add_friend_link(&link).as_deref(), Some(CODE));
    }

    #[test]
    fn parses_custom_scheme_link() {
        let link = build_add_friend_scheme_link(CODE);
        assert_eq!(parse_add_friend_link(&link).as_deref(), Some(CODE));
    }

    #[test]
    fn parses_legacy_custom_scheme_without_path() {
        let link = format!("{DEEP_LINK_SCHEME}://{CODE}");
        assert_eq!(parse_add_friend_link(&link).as_deref(), Some(CODE));
    }

    #[test]
    fn parses_dev_http_link() {
        let link = format!("http://{DEEP_LINK_HOST}/add/{CODE}");
        assert_eq!(parse_add_friend_link(&link).as_deref(), Some(CODE));
    }

    #[test]
    fn rejects_unrelated_url() {
        assert_eq!(parse_add_friend_link("https://example.com/add/1234"), None);
    }

    #[test]
    fn rejects_non_hex_code() {
        assert_eq!(parse_add_friend_link("onlyfriends://add/not-hex!!"), None);
    }

    #[test]
    fn rejects_odd_length_hex() {
        assert_eq!(parse_add_friend_link("onlyfriends://add/abc"), None);
    }

    #[test]
    fn rejects_plain_garbage() {
        assert_eq!(parse_add_friend_link("not a link at all"), None);
    }

    #[test]
    fn finds_add_friend_arg_among_argv() {
        let link = build_add_friend_scheme_link(CODE);
        let args = vec![
            "/usr/bin/onlyfriends".to_string(),
            "--some-flag".to_string(),
            link,
        ];
        assert_eq!(find_add_friend_arg(args).as_deref(), Some(CODE));
    }

    #[test]
    fn finds_no_add_friend_arg_when_absent() {
        let args = vec!["/usr/bin/onlyfriends".to_string(), "--headless".to_string()];
        assert_eq!(find_add_friend_arg(args), None);
    }
}
