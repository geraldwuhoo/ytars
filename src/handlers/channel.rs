use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoListModel, VideoType},
    util::_default_video_type,
};

#[derive(Debug, Template)]
#[template(path = "channel.html")]
struct ChannelTemplate {
    channel: ChannelModel,
    videos: Vec<VideoListModel>,
    video_type: VideoType,
}

#[derive(Debug, Deserialize)]
pub struct ChannelParams {
    #[serde(default = "_default_video_type")]
    video_type: VideoType,
}

#[get("/channel/{uri}")]
pub async fn channel_handler(
    params: web::Query<ChannelParams>,
    uri: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let channel_id = uri.to_string();
    let video_type = params.video_type;
    let videos = sqlx::query_as!(
        VideoListModel,
        r#"SELECT
            id,
            title,
            upload_date,
            duration_string,
            channel_id,
            video_type AS "video_type: VideoType",
            view_count
        FROM video
        WHERE channel_id = $1 AND video_type = $2
        ORDER BY upload_date DESC;"#,
        channel_id,
        video_type as VideoType,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let channel = sqlx::query_as!(
        ChannelModel,
        "SELECT * FROM channel WHERE id = $1;",
        channel_id,
    )
    .fetch_one(pool.as_ref())
    .await?;

    let ytchannel = ChannelTemplate {
        channel,
        videos,
        video_type,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(ytchannel.render()?))
}
