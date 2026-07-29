# `.well-known` files for Universal Links / App Links

These files are the source of truth for two things served by
`onlyfriends-relay` (`relay/src/landing.rs`, embedded via `include_str!`
so the served content can't drift from what's checked in here):

- `https://onlyfriends.app/.well-known/apple-app-site-association`
  (no file extension — required exactly as-is by iOS)
- `https://onlyfriends.app/.well-known/assetlinks.json`

The relay must actually be deployed at `onlyfriends.app` (or wherever
`DEEP_LINK_HOST` points) for these to be reachable at the right URL — see
the root `README.md` for the relay's release/deploy process. If the relay
lives at a different host than the marketing domain, these two routes
(and `/add/*`, below) need to be reverse-proxied from `onlyfriends.app`
to the relay.

iOS and Android fetch these at install time to verify that this app is
actually authorized to handle links for `onlyfriends.app`, before enabling
"open directly in app" behavior for Universal Links / App Links. Without
them, links still work but always show an "Open in browser or app?" prompt
instead of opening directly.

Everything else needed on the *app* side (CFBundleURLTypes, associated
domains entitlement, Android intent-filters) is generated automatically by
`dx bundle` from the `[deep_links]` section in `ui/mobile/Dioxus.toml` — see
the comments there. These two files are the only pieces that can't come
from the Dioxus build, since they need to be served from your web domain,
not bundled into the app.

## Placeholders — update before shipping

| Placeholder | File | Replace with |
|---|---|---|
| `onlyfriends.app` | both (implicitly, via hosting path) | your real production domain — also update `client_core::deep_link::DEEP_LINK_HOST` and `ui/mobile/Dioxus.toml`'s `[deep_links]` section to match |
| `TEAMID` | `apple-app-site-association` | your Apple Developer Team ID |
| `com.onlyfriends.app` | both | your real bundle identifier / Android `applicationId` — keep in sync with `ui/mobile/Dioxus.toml`'s `[bundle] identifier` |
| `SHA256_CERT_FINGERPRINT_PLACEHOLDER` | `assetlinks.json` | your release signing certificate's SHA-256 fingerprint, e.g. via `keytool -list -v -keystore <keystore> \| grep SHA256` |

## Known gaps (see `DEEP_LINK_PLAN.md`)

- The `/add/{code}` fallback landing page (`relay/src/landing.rs`) exists
  but has no real download links yet, since there's no published App
  Store / Play Store listing — it just tries the `onlyfriends://` custom
  scheme and shows a "not published yet" message.
- Desktop (Linux/macOS/Windows) OS-level protocol registration is not yet
  wired up to an equivalent official mechanism — being revisited in a
  follow-up.
