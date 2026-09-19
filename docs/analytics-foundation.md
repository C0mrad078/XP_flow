# Phase 8 — Analytics Foundation

Phase 8 introduces persisted analytics snapshots without changing the publishing execution model.

## Data model

`publication_metric_snapshots` and `channel_metric_snapshots` retain timestamped provider observations. Metric columns are nullable: `0` is a provider-reported zero, while `NULL` means the value was not supplied. `availability` distinguishes supported, unsupported, unavailable and failed observations. Queries are bounded to the requested time range and at most 1,000 rows.

Audience metrics (views, likes, comments, followers) are kept separate from XP FLOW operational metrics such as publication status, retries and scheduler activity. No audience metric is inferred from local publishing records.

## Provider capabilities

YouTube currently uses the authenticated `videos.list?part=statistics` endpoint for publication views, likes and comments. The adapter preserves missing fields and remote-not-found responses. Channel statistics, shares and richer watch-time analytics are not claimed by this integration yet.

TikTok and Kwai report an explicit unsupported capability result because the current desktop integrations do not expose a reliable analytics endpoint under their granted permissions. They are not represented as zeroes and no scraping is used.

## Synchronization

The Analytics service provides persisted publication snapshots, per-account sync state, a manual refresh command, and a centralized 15-minute background sweep. The sweep is bounded to 20 recent published items per supported account, skips unsupported/disconnected or rate-limited accounts, and runs independently of publishing workers. Refresh uses the existing credential acquisition path, so token refresh and provider authentication remain centralized. Analytics failures are returned independently of publication state. Channel-level provider refresh remains a future extension.

The UI shows the latest persisted capture time, unavailable metrics and a per-publication Refresh action. Cached values are not presented as live data.

## Privacy and release validation

Snapshots contain provider identifiers and numeric observations only. Access and refresh tokens are never persisted in analytics tables or returned by analytics commands. Engineering adapter tests do not prove live provider access. RG-01 through RG-04 remain pending in [release-validation.md](release-validation.md). Live analytics verification, when required for a release, is tracked separately as RG-05.
