-- Add
-- channel thumbnail table

DROP TABLE IF EXISTS channel_thumbnail;
CREATE TABLE "channel_thumbnail" (
    id varchar(64) NOT NULL PRIMARY KEY,
    thumbnail bytea NOT NULL
);

DROP TABLE IF EXISTS video_thumbnail;
CREATE TABLE "video_thumbnail" (
    id varchar(64) NOT NULL PRIMARY KEY,
    thumbnail bytea NOT NULL
);
