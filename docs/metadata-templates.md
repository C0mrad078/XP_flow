# Metadata templates

How XP FLOW decides what title, description and hashtags actually get sent to a provider —
`application::metadata_template_service::MetadataTemplateService`, the missing piece Phase 5 documented and Phase
5.1 built.

## The precedence ladder

For each of title and description independently, and for hashtags via the analogous `HashtagSet` ladder:

```text
1. Publication override        — literal text/list typed for this one publication
2. Pinned specific template    — "Select specific template" in the editor
3. Channel + Platform template
4. Channel default              (channel set, no platform)
5. Workspace + Platform default (no channel, platform set)
6. Workspace default            (no channel, no platform)
7. The publication's own raw title/description/hashtags fields
```

Rung 7 is exactly the `Publication.title`/`description`/`hashtags` fields that have existed since Phase 1 —
nothing about them changed. A publication that has never touched the template system resolves straight to rung 7
and behaves identically to before this phase. Every rung above it is additive.

The same ladder governs hashtags, resolved through `HashtagSet` instead of `MetadataTemplate` (`kind` doesn't
apply — a `HashtagSet` has no title/description split).

## Data model

Three mechanisms, matching the editor's "Use automatic template / Select specific template / Publication
override" selector:

- **Scope-based templates** (`metadata_templates`, `hashtag_sets` — unchanged since Phase 5): rows scoped by
  `(workspace_id, channel_id?, platform?)`. Creating a second template for the _exact_ same scope tuple and
  `kind` is rejected (`MetadataTemplateService::ensure_scope_is_free`) — precedence resolution would otherwise be
  ambiguous about which one wins.
- **Publication overrides** (`publications.title_override` / `description_override` / `hashtags_override`,
  Phase 5.1): literal text/list for one publication, still rendered through `{variables}`.
- **Publication pins** (`publications.title_template_id` / `description_template_id` / `hashtag_set_id`, Phase
  5.1): points at one specific template/set, bypassing the scope chain. `ON DELETE SET NULL` means deleting a
  template can never leave a publication pointing at a row that no longer exists — resolution just falls through
  to the next rung, exactly as if the pin had never been set. This is also why template/hashtag-set deletion needs
  no extra bookkeeping: nothing else in the schema references a template except through this one FK, and it's
  already safe by construction.

## Variables

```text
{title}       — the publication's own title
{video_title} — the underlying video's display title (distinct from {title})
{filename}    — the video's original filename
{source}      — the video source's name
{channel}     — the channel's name
{platform}    — the platform's display name
{date}        — the publication's scheduled date (YYYY-MM-DD), or today if unscheduled
{hashtags}    — the already-resolved hashtag list, space-joined
```

`{hashtags}` is resolved _before_ title/description rendering, so a title template can legitimately end with
`{hashtags}` and see the real resolved list, not an empty placeholder. An unrecognized `{placeholder}` (a typo, or
a variable that doesn't exist) is left untouched in the output rather than silently dropped — visible in preview,
never published as an accidental blank.

## Execution snapshot

`PublishingEngineService::execute_inner` calls `MetadataTemplateService::render_for_publication` — the exact same
resolution function the live preview (`preview_publication_metadata`) and validation
(`get_publication_readiness`'s metadata check) use — and freezes the result onto `Publication.rendered_metadata`
the instant execution starts. A template edited afterward can never retroactively change what an in-flight or
already-executed attempt claims it sent; only a _future_ scheduled publication re-renders with the new template.
Metadata becomes immutable for a given publication exactly at that freeze point — before it, every preview call is
live; after it, `rendered_metadata` is the permanent record of what was actually sent, independent of anything
this service resolves later.

## Provider options

`Publication.metadata_overrides.provider_options_override` is a JSON object holding the provider-specific fields
each uploader's `PlatformPublisher::initialize_upload` already knows how to read (YouTube's `privacy_status`/
`category_id`, TikTok's `privacy_level`/`disable_duet`/`disable_stitch`/`disable_comment`/
`video_cover_timestamp_ms`, Kwai's `cover`/`caption`/`stero_type`). There is no separate template system for these
— they're per-publication only, not resolved through the channel/workspace ladder, since a privacy level or
comment toggle is inherently a per-post decision rather than something that makes sense as a reusable template.

## Validation

`MetadataTemplateService::validate` renders a publication's current metadata and runs it through the target
platform's real `PlatformPublisher::validate_metadata` — the exact same check `execute_inner` runs before ever
transmitting anything. `PublishError::InvalidMetadata`'s free-text detail is classified into a typed
`MetadataValidationIssueCode` (`TITLE_TOO_LONG`, `DESCRIPTION_TOO_LONG`, `CAPTION_TOO_LONG`,
`MISSING_REQUIRED_FIELD`, `INVALID_PROVIDER_OPTION`) via best-effort substring matching on the message each check
already produces — the validation _rules_ themselves are never duplicated in the frontend, only the resulting
message is classified for display.
