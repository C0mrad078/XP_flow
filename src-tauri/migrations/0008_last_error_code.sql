-- Phase 5.1 section 66/67: a stable, typed signal alongside the existing
-- free-text `last_error`, so the frontend can reliably detect
-- UNKNOWN_REMOTE_RESULT specifically (an ambiguous remote outcome) and
-- refuse to offer blind retry as the primary action for it, without
-- pattern-matching on prose that could change wording later.
ALTER TABLE publications ADD COLUMN last_error_code TEXT;
