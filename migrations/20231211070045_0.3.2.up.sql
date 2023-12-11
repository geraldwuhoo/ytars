-- Add
-- likes column in video
-- dislikes column in video

ALTER TABLE video
DROP COLUMN IF EXISTS likes;
ALTER TABLE video
ADD COLUMN likes integer;

ALTER TABLE video
DROP COLUMN IF EXISTS dislikes;
ALTER TABLE video
ADD COLUMN dislikes integer;
