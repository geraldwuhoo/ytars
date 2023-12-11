-- Remove
-- likes column in video
-- dislikes column in video

ALTER TABLE video
DROP COLUMN IF EXISTS likes;

ALTER TABLE video
DROP COLUMN IF EXISTS dislikes;
