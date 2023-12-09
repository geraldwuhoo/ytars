-- Remove:
-- subscriber count column to channels
-- view count column to videos

ALTER TABLE channel
DROP COLUMN IF EXISTS channel_follower_count;

DROP INDEX IF EXISTS video_view_count_idx;

ALTER TABLE video
DROP COLUMN IF EXISTS view_count;
