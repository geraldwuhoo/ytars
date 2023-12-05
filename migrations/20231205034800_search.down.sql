-- Remove:
-- Search support

DROP TRIGGER IF EXISTS tsvectorupdate ON video;

DROP FUNCTION IF EXISTS video_tsvector_trigger;

DROP INDEX IF EXISTS document_idx;

ALTER TABLE video DROP COLUMN IF EXISTS document;
