-- Remove:
-- subscriber count column to channels

ALTER TABLE channel
DROP COLUMN IF EXISTS channel_follower_count;
