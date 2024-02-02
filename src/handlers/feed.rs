use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use askama::Template;
use log::debug;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{VideoChannelJoinModel, VideoType},
    util::{_default_page, _default_video_type, get_cookie_value_bool, get_cookie_value_i64},
};

#[derive(Debug, Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
    videos: Vec<VideoChannelJoinModel>,
    video_type: VideoType,
    show_thumbnails: bool,
    likes_dislikes_on_channel_page: bool,
    page: i64,
    page_size: i64,
    max_results_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct FeedParams {
    #[serde(default = "_default_video_type")]
    video_type: VideoType,
    #[serde(default = "_default_page")]
    page: i64,
}

#[get("/feed")]
pub async fn feed_handler(
    req: HttpRequest,
    params: web::Query<FeedParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let show_thumbnails = get_cookie_value_bool(&req, "thumbnails_for_feed")?
        || get_cookie_value_bool(&req, "thumbnails_for_all_videos")?;
    let likes_dislikes_on_channel_page =
        get_cookie_value_bool(&req, "likes/dislikes_on_channel_page")?;
    let video_type = params.video_type;
    let page_size = get_cookie_value_i64(&req, "videos_per_page")?;
    let page = params.page;
    let max_results_count = 200;

    debug!("Getting page {} size {} for feed", page, page_size);

    let offset = std::cmp::min(page * page_size, max_results_count);
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
            filestem,
            likes,
            dislikes
        FROM video
        INNER JOIN channel ON video.channel_id = channel.id
        WHERE video_type = $1
        ORDER BY upload_date DESC
        OFFSET $2
        LIMIT $3;"#,
        video_type as VideoType,
        offset,
        std::cmp::max(std::cmp::min(max_results_count - offset, page_size), 0),
    )
    .fetch_all(pool.get_ref())
    .await?;

    let feed = FeedTemplate {
        videos,
        video_type,
        show_thumbnails,
        likes_dislikes_on_channel_page,
        page,
        page_size,
        max_results_count,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(feed.render()?))
}
