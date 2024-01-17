use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{VideoChannelJoinModel, VideoType},
    util::{_default_count, _default_video_type, get_show_thumbnails},
};

#[derive(Debug, Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
    videos: Vec<VideoChannelJoinModel>,
    video_type: VideoType,
    show_thumbnails: bool,
}

#[derive(Debug, Deserialize)]
pub struct FeedParams {
    #[serde(default = "_default_count")]
    count: i64,
    #[serde(default = "_default_video_type")]
    video_type: VideoType,
}

#[get("/feed")]
pub async fn feed_handler(
    req: HttpRequest,
    params: web::Query<FeedParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let show_thumbnails = get_show_thumbnails(&req)?;
    let video_type = params.video_type;
    let videos = sqlx::query_as!(
        VideoChannelJoinModel,
        r#"SELECT
            video.id,
            title,
            upload_date,
            duration_string,
            channel.id AS channel_id,
            channel.name,
            video_type AS "video_type: VideoType",
            view_count,
            channel.sanitized_name AS channel_sanitized_name,
            filestem
        FROM video
        INNER JOIN channel ON video.channel_id = channel.id
        WHERE video_type = $1
        ORDER BY upload_date DESC
        LIMIT $2;"#,
        video_type as VideoType,
        params.count,
    )
    .fetch_all(pool.get_ref())
    .await?;

    let feed = FeedTemplate {
        videos,
        video_type,
        show_thumbnails,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(feed.render()?))
}
