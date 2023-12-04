use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::YtarsError,
    model::{VideoChannelJoinModel, VideoType, _default_video_type},
};

#[derive(Debug, Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
    videos: Vec<VideoChannelJoinModel>,
}

#[derive(Debug, Deserialize)]
pub struct FeedParams {
    #[serde(default = "_default_count")]
    count: i64,
    #[serde(default = "_default_video_type")]
    video_type: VideoType,
}

const fn _default_count() -> i64 {
    100
}

#[get("/feed")]
pub async fn feed_handler(
    params: web::Query<FeedParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let videos = sqlx::query_as!(
        VideoChannelJoinModel,
        r#"SELECT
            video.id,
            title,
            upload_date,
            duration_string,
            channel.id AS channel_id,
            channel.name,
            video_type AS "video_type: VideoType"
        FROM video
        INNER JOIN channel ON video.channel_id = channel.id
        WHERE video_type = $1
        ORDER BY upload_date DESC
        LIMIT $2;"#,
        params.video_type as VideoType,
        params.count,
    )
    .fetch_all(pool.get_ref())
    .await?;

    let ytchannel = FeedTemplate { videos };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(ytchannel.render()?))
}
