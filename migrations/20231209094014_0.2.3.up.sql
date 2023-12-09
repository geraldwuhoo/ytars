-- Add:
-- subscriber count column to channels
-- view count column to videos

ALTER TABLE channel
DROP COLUMN IF EXISTS channel_follower_count;
ALTER TABLE channel
ADD COLUMN channel_follower_count integer DEFAULT 0 NOT NULL;

ALTER TABLE video
DROP COLUMN IF EXISTS view_count;
ALTER TABLE video
ADD COLUMN view_count bigint DEFAULT 0 NOT NULL;
