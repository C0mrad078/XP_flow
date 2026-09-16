-- Phase 5.1: per-publication metadata resolution controls (section 5/40
-- of the Phase 5.1 brief).
--
-- Every new column is nullable and defaults to NULL, so an existing
-- publication's effective metadata is completely unchanged by this
-- migration: `title`/`description`/`hashtags` (added in earlier phases)
-- remain the lowest-precedence "Video / Publication Raw Fields" rung of
-- the resolution ladder documented in docs/metadata-templates.md.
--
-- Two independent mechanisms per field (title/description/hashtags),
-- matching the editor's three-mode "Use automatic template / Select
-- specific template / Publication override" selector:
--   * `*_override`      — literal text/list for this one publication,
--                         the highest-precedence rung.
--   * `*_template_id` / `hashtag_set_id` — pins resolution to one exact
--                         template/set rather than the channel/workspace
--                         precedence chain. `ON DELETE SET NULL` means
--                         deleting a template can never leave a
--                         publication pointing at a row that no longer
--                         exists — resolution simply falls through to
--                         the next rung, exactly as if it had never been
--                         pinned.
--
-- `provider_options_override` lets the metadata editor's per-provider
-- tabs (YouTube privacy/category, TikTok privacy_level/duet/stitch/
-- comment, Kwai cover/caption) set real values instead of every
-- publication silently using each uploader's hardcoded conservative
-- default.
ALTER TABLE publications ADD COLUMN title_override TEXT;
ALTER TABLE publications ADD COLUMN description_override TEXT;
ALTER TABLE publications ADD COLUMN hashtags_override TEXT;
ALTER TABLE publications ADD COLUMN title_template_id TEXT REFERENCES metadata_templates(id) ON DELETE SET NULL;
ALTER TABLE publications ADD COLUMN description_template_id TEXT REFERENCES metadata_templates(id) ON DELETE SET NULL;
ALTER TABLE publications ADD COLUMN hashtag_set_id TEXT REFERENCES hashtag_sets(id) ON DELETE SET NULL;
ALTER TABLE publications ADD COLUMN provider_options_override TEXT;
