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
    pub short: bool,
    pub channel_id: String,
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
}
