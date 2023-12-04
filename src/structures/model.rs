use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
#[allow(non_snake_case)]
pub struct ChannelModel {
    pub id: String,
    #[serde(rename = "channel")]
    pub name: String,
    #[serde(skip_deserializing)]
    pub sanitized_name: String,
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "video_type", rename_all = "lowercase")]
pub enum VideoType {
    Video,
    Short,
    Stream,
}

pub const fn _default_video_type() -> VideoType {
    VideoType::Video
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case)]
pub struct VideoModel {
    pub id: String,
    pub title: String,
    pub filename: String,
    pub filestem: String,
    pub upload_date: Date,
    pub duration_string: String,
    pub description: Option<String>,
    pub channel_id: String,
    pub video_type: VideoType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VideoJson {
    pub id: String,
    pub title: String,
    pub upload_date: String,
    pub duration_string: String,
    pub aspect_ratio: f32,
    pub description: Option<String>,
    pub channel_id: String,
    #[serde(default)]
    pub was_live: bool,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case)]
pub struct VideoListModel {
    pub id: String,
    pub title: String,
    pub upload_date: Date,
    pub duration_string: String,
    pub channel_id: String,
    pub video_type: VideoType,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case)]
pub struct VideoChannelJoinModel {
    pub id: String,
    pub title: String,
    pub upload_date: Date,
    pub duration_string: String,
    pub channel_id: String,
    pub name: String,
    pub video_type: VideoType,
}