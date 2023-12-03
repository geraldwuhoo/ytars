-- Add:
-- convert short column to enum for video type (video, short, stream)

DROP TYPE IF EXISTS video_type;
CREATE TYPE video_type AS ENUM('video', 'short', 'stream');

ALTER TABLE video
DROP COLUMN IF EXISTS short;

ALTER TABLE video
DROP COLUMN IF EXISTS video_type;
ALTER TABLE video
ADD COLUMN video_type video_type DEFAULT 'video' NOT NULL;

DROP INDEX IF EXISTS video_type_idx;
CREATE INDEX video_type_idx ON video USING HASH (video_type);
