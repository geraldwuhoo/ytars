-- Add up migration script here

DROP TABLE IF EXISTS channel;
CREATE TABLE "channel" (
    id varchar(64) NOT NULL PRIMARY KEY,
    name varchar(64) NOT NULL UNIQUE,
    sanitized_name varchar(64) NOT NULL UNIQUE,
    description text
);

DROP TABLE IF EXISTS video;
CREATE TABLE "video" (
    id varchar(64) NOT NULL PRIMARY KEY,
    title text NOT NULL,
    filename text NOT NULL,
    filestem text NOT NULL,
    upload_date date NOT NULL,
    duration_string text NOT NULL,
    description text,
    short boolean NOT NULL,
    channel_id varchar(64) NOT NULL REFERENCES channel (id)
);

DROP INDEX IF EXISTS video_channel_idx;
CREATE INDEX video_channel_idx ON video USING HASH (channel_id);

DROP INDEX IF EXISTS video_upload_date_idx;
CREATE INDEX video_upload_date_idx ON video (upload_date);

DROP INDEX IF EXISTS video_filestem_idx;
CREATE INDEX video_filestem_idx ON video USING HASH (filestem);

DROP INDEX IF EXISTS channel_name_idx;
CREATE INDEX channel_name_idx ON channel (name);

