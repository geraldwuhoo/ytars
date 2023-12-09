-- Remove:
-- subscriber count column to channels

ALTER TABLE channel
DROP COLUMN IF EXISTS channel_follower_count;

ALTER TABLE video
DROP COLUMN IF EXISTS view_count;
