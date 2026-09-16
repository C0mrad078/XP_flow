# Provider capabilities

How XP FLOW turns a provider's raw OAuth scope strings into the typed permissions the rest of the app reasons
about — `domain::capability`. The UI, Queue readiness signals, and Dashboard health summary never inspect a raw
scope string directly; they only ever check `PlatformAccount.capabilities`.

## Why a separate `Capability` enum instead of raw scopes

A `PlatformAccount` can be `Connected` (the OAuth flow succeeded, the account is usable) while still lacking
`UploadVideo` (the user never granted, or the app never requested, publish rights). Conflating "connected" with
"can publish" was an explicit thing to avoid — see `docs/platform-authentication.md` §4. The `Capability` enum is
the single vocabulary every screen uses:

```
ReadProfile · UploadVideo · ReadVideoStatus · ReadMetrics · ReadComments · WriteComments
```

`map_scopes_to_capabilities(platform, scopes) -> Vec<Capability>` (`src-tauri/src/domain/capability.rs`) is the
**only** place in the codebase that ever has to know what a raw provider scope string means. It is pure — no I/O,
tested directly against literal scope strings.

## Scope → capability mapping

### YouTube (Google scope URIs)

| Scope | Capability granted |
|---|---|
| `openid`, `.../auth/userinfo.profile` | `ReadProfile` |
| `.../auth/youtube.readonly`, `.../auth/youtube.force-ssl` | `ReadVideoStatus`, `ReadMetrics` |
| `.../auth/youtube.upload`, `.../auth/youtube.force-ssl` | `UploadVideo` |
| `.../auth/youtube.force-ssl` | also `ReadComments`, `WriteComments` |

### TikTok

| Scope | Capability granted |
|---|---|
| `user.info.basic`, `user.info.profile` | `ReadProfile` |
| `video.list` | `ReadVideoStatus` |
| `video.publish`, `video.upload` | `UploadVideo` |

### Kwai

| Scope | Capability granted |
|---|---|
| `user_info` | `ReadProfile` |
| `video_upload` | `UploadVideo` |
| `video_status` | `ReadVideoStatus` |

## Default requested scopes (least privilege)

`default_requested_scopes(platform)` — what XP FLOW actually asks for today:

| Platform | Scopes requested | Capabilities this yields |
|---|---|---|
| YouTube | `openid`, `.../auth/userinfo.profile`, `.../auth/youtube.readonly` | `ReadProfile`, `ReadVideoStatus`, `ReadMetrics` |
| TikTok | `user.info.basic` | `ReadProfile` |
| Kwai | `user_info` | `ReadProfile` |

**None of these ever grant `UploadVideo`** — enforced by a test
(`capability::tests::default_requested_scopes_never_include_publishing`) that runs the default scopes for every
`Platform` through the mapping function and asserts `UploadVideo` is absent. Connecting an account in this phase
never silently acquires publish rights; that's a deliberate Phase 5 decision, not an oversight.

## Implementation complete ≠ provider approved

Two separate facts, and XP FLOW is careful not to conflate them in its own UI:

1. **"XP FLOW's code correctly implements this provider's OAuth flow and scope handling."** True for all three
   providers as of this phase — verified against fakes/mocks, see the Known Limitations section of
   `docs/platform-authentication.md`.
2. **"This specific XP FLOW application has been approved by the provider to use publish-level scopes in
   production."** A fact XP FLOW cannot verify or fake — it depends entirely on each provider's own developer
   console and review process, external to this codebase.

Concretely, before real publishing (Phase 5) can go live for real users:

- **TikTok** — the Content Posting API's `video.publish` scope requires a separate app audit through TikTok's
  developer console before it can be used outside a small set of approved test users. An app that only requests
  `user.info.basic` today (as this phase does) has nothing pending review yet.
- **Kwai** — publishing scopes on Kwai's Open Platform similarly require partner/app approval beyond basic OAuth
  registration.
- **YouTube** — the Data API's `youtube.upload` scope is subject to Google's own OAuth verification process for
  apps requesting sensitive/restricted scopes once past a small quota of test users.

`src/features/integrations/provider-card.tsx` surfaces this as a static, honest note per provider (not a live
status check XP FLOW has no way to perform) rather than implying "connected" means "ready to publish."
