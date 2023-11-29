use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{errors::YtarsError, model::VideoChannelJoinModel};

#[derive(Debug, Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
    videos: Vec<VideoChannelJoinModel>,
}

#[derive(Debug, Deserialize)]
pub struct FeedParams {
    count: Option<i64>,
    shorts: Option<bool>,
}

#[get("/feed")]
pub async fn feed_handler(
    params: web::Query<FeedParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let videos = sqlx::query_as!(
        VideoChannelJoinModel,
        "SELECT video.id, title, upload_date, duration_string, channel.id AS channel_id, channel.name
        FROM video
        INNER JOIN channel ON video.channel_id = channel.id
        WHERE short = $1
        ORDER BY upload_date DESC
        LIMIT $2;",
        params.shorts.unwrap_or(false),
        params.count.unwrap_or(100),
    )
    .fetch_all(pool.get_ref())
    .await?;

    let ytchannel = FeedTemplate { videos };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(ytchannel.render()?))
}
