# YouTube Connection

XP FLOW uses Google's installed/desktop OAuth client flow. The system browser is opened only after XP FLOW binds an
ephemeral `127.0.0.1` port. The exact redirect URI, including its port and `/oauth/youtube/callback` path, is used for
both authorization and token exchange. PKCE S256 and a per-attempt state value protect the callback; the listener is
single-use and closes after success, cancellation, error, or the five-minute timeout.

## Google project setup

The configured client must be an OAuth **Desktop app** client. The Google project must have YouTube Data API v3
enabled. XP FLOW uses `YOUTUBE_CLIENT_ID` when supplied, otherwise the built-in public desktop client ID. A desktop
client secret is optional and is never required for the PKCE flow; if `YOUTUBE_CLIENT_SECRET` is configured it is sent
only to the token endpoint according to the existing client configuration.

The connection requests `openid`, profile, `youtube.readonly`, and `youtube.upload`. The upload permission is required
because a connected account is also used by the publishing engine. Existing accounts that were connected with only
read scopes must reconnect and approve the expanded scope set.

## Runtime behavior

The flow is: Tauri command → native Rust auth service → loopback listener → system browser → Google callback → code
exchange → `/youtube/v3/channels?part=snippet&mine=true` → OS keychain credential storage → SQLite account state.
The stable YouTube channel ID is stored as the provider identity. An authorized Google account with no usable YouTube
channel fails clearly and is not marked as a functional publishing account.

Tokens are stored only in the existing OS keychain. Authorization codes, PKCE verifiers, access tokens and refresh
tokens are never logged or returned to the frontend. Google may omit a refresh token on a later refresh; XP FLOW keeps
the previously stored refresh token in that case.

## Troubleshooting

- **Browser did not open:** retry after confirming the system default browser is configured; XP FLOW reports a failed
  connection instead of leaving the dialog loading.
- **Authorization cancelled or denied:** the attempt ends and Connect can be started again.
- **Callback timed out/state mismatch:** start a fresh attempt; the listener and state are not reused.
- **Permission or API errors:** confirm YouTube Data API v3 is enabled and approve all requested scopes.
- **Packaged app configuration:** provide `YOUTUBE_CLIENT_ID` through the packaged runtime environment when using a
  project-specific client. Do not put secrets in frontend files or source control.

Reconnect updates the existing stable account row without duplicating it. Disconnect revokes credentials on a best-effort
basis, removes local keychain material, and preserves publication and analytics history.
