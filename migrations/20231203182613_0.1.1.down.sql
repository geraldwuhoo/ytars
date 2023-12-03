-- Remove:
-- convert short column to enum for video type (video, short, stream)

DROP INDEX IF EXISTS video_type_idx;

ALTER TABLE video
DROP COLUMN IF EXISTS video_type;

DROP TYPE IF EXISTS video_type;

ALTER TABLE video
ADD COLUMN short boolean DEFAULT false NOT NULL;
