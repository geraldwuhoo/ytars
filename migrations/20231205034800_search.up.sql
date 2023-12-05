-- Add:
-- Search support
-- Example search query:
-- SELECT title FROM video WHERE document @@ plainto_tsquery('space') ORDER BY ts_rank(document, plainto_tsquery('space')) DESC LIMIT 50;

ALTER TABLE video DROP COLUMN IF EXISTS document;
ALTER TABLE video ADD COLUMN document tsvector;
UPDATE video
SET document =
    setweight(to_tsvector('english', title), 'A') ||
    setweight(to_tsvector('english', COALESCE(description, '')), 'B');

DROP INDEX IF EXISTS document_idx;
CREATE INDEX document_idx ON video USING GIN (document);

DROP FUNCTION IF EXISTS video_tsvector_trigger;
CREATE FUNCTION video_tsvector_trigger() RETURNS trigger AS $$
BEGIN
    new.document :=
    setweight(to_tsvector('english', new.title), 'A') ||
    setweight(to_tsvector('english', COALESCE(new.description, '')), 'B');
    RETURN new;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tsvectorupdate ON video;
CREATE TRIGGER tsvectorupdate BEFORE INSERT OR UPDATE
    ON video FOR EACH ROW EXECUTE PROCEDURE video_tsvector_trigger();
