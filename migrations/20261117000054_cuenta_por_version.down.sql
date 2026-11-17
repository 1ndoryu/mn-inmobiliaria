/* [30AA-13] Deshacer: la cuenta deja de estar atada a una versión. */

DROP INDEX IF EXISTS idx_facebook_version;
ALTER TABLE facebook DROP COLUMN IF EXISTS version;
