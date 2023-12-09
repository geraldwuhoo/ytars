-- Add:
-- subscriber count column to channels

ALTER TABLE channel
DROP COLUMN IF EXISTS channel_follower_count;
ALTER TABLE channel
ADD COLUMN channel_follower_count integer DEFAULT 0 NOT NULL;
