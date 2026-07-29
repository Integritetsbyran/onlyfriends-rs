//! "Add friend" deep-link fallback landing page, and the `.well-known`
//! files needed for iOS Universal Links / Android App Links.
//!
//! When a scanned QR / shared `https://<host>/add/<code>` link isn't
//! intercepted directly by the app (not installed, or link verification
//! failed), the OS falls back to opening it in a browser — this module is
//! what gets served in that case.
//!
//! See `well-known/README.md` and `client_core::deep_link` for the broader
//! deep-link design. The scheme/path constants below intentionally
//! duplicate the tiny string literals from `client_core::deep_link` rather
//! than depending on that crate (and its crypto/HTTP-client dependency
//! tree) from this server binary — keep them in sync if either changes.

use axum::{
    extract::Path,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse, Response},
};

/// Custom URL scheme the app registers — see `client_core::deep_link::DEEP_LINK_SCHEME`.
const DEEP_LINK_SCHEME: &str = "onlyfriends";

/// AASA file content, embedded at compile time so it can't drift from the
/// checked-in template. See `well-known/README.md` for the placeholders
/// that need updating before shipping.
const APPLE_APP_SITE_ASSOCIATION: &str =
    include_str!("../../well-known/apple-app-site-association");

/// Android Digital Asset Links file content, embedded the same way.
const ASSET_LINKS_JSON: &str = include_str!("../../well-known/assetlinks.json");

/// `GET /add/{code}` — the "add friend" landing page.
///
/// Only ever reached when the OS didn't hand the link straight to the app
/// (not installed, or Universal Link/App Link verification failed). Tries
/// a best-effort redirect to the custom scheme, then falls back to a
/// static message since there's no public app listing yet.
///
/// `code` is validated as hex before being embedded in the page — it's
/// untrusted input from the URL, and this is a simple substring
/// interpolation rather than a templating engine, so no escaping would
/// otherwise happen.
pub async fn add_friend_landing(Path(code): Path<String>) -> Response {
    if !is_hex(&code) {
        return (StatusCode::BAD_REQUEST, Html(PAGE_INVALID)).into_response();
    }

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Add friend · OnlyFriends</title>
<style>{STYLE}</style>
</head>
<body>
<div class="card">
<h1>Add friend on OnlyFriends</h1>
<p>Opening the app…</p>
<p class="hint">
OnlyFriends is still in early development and isn't published on the
App Store or Play Store yet. If the app didn't open, ask whoever shared
this link how to install it, then scan their code again.
</p>
</div>
<script>
window.location.replace("{DEEP_LINK_SCHEME}://add/{code}");
</script>
</body>
</html>"#
    ))
    .into_response()
}

/// `GET /.well-known/apple-app-site-association`
///
/// Must be served with an `application/json` content type and without a
/// file extension — iOS fetches this at install time to verify the app is
/// authorized to handle Universal Links for this domain.
pub async fn apple_app_site_association() -> Response {
    json_response(APPLE_APP_SITE_ASSOCIATION)
}

/// `GET /.well-known/assetlinks.json`
///
/// Android's equivalent verification file for App Links.
pub async fn asset_links_json() -> Response {
    json_response(ASSET_LINKS_JSON)
}

fn json_response(body: &'static str) -> Response {
    ([(CONTENT_TYPE, "application/json")], body).into_response()
}

/// Same hex validation as `client_core::deep_link::parse_add_friend_link`
/// (kept independent for the dependency-tree reasons above) — anything
/// that fails this check is never interpolated into the response.
fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.len().is_multiple_of(2) && s.chars().all(|c| c.is_ascii_hexdigit())
}

const STYLE: &str = "
body { font-family: system-ui, sans-serif; background: #0f1116; color: #fff;
       display: flex; align-items: center; justify-content: center;
       min-height: 100vh; margin: 0; padding: 24px; }
.card { max-width: 420px; text-align: center; }
h1 { font-size: 1.25rem; }
.hint { color: #9aa0ac; font-size: 0.9rem; }
";

const PAGE_INVALID: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Invalid link · OnlyFriends</title>
</head>
<body>
<p>This link doesn't contain a valid friend code.</p>
</body>
</html>"#;
