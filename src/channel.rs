use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use sqlx::PgPool;

use crate::{
    errors::YtarsError,
    model::{ChannelModel, VideoModel},
};

#[derive(Debug, Template)]
#[template(path = "channel.html")]
struct ChannelTemplate {
    channel: ChannelModel,
    videos: Vec<VideoModel>,
    shorts: bool,
}

async fn get_channel_page(
    channel_id: String,
    shorts: bool,
    pool: &PgPool,
) -> Result<String, YtarsError> {
    let videos = sqlx::query_as!(
        VideoModel,
        "SELECT * FROM video
        WHERE channel_id = $1 AND short = $2
        ORDER BY upload_date DESC;",
        channel_id,
        shorts,
    )
    .fetch_all(pool)
    .await?;

    let channel = sqlx::query_as!(
        ChannelModel,
        "SELECT * FROM channel WHERE id = $1;",
        channel_id,
    )
    .fetch_one(pool)
    .await?;

    let ytchannel = ChannelTemplate {
        channel,
        videos,
        shorts,
    };
    Ok(ytchannel.render()?)
}

#[get("/shorts/{uri}")]
pub async fn shorts_handler(
    uri: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let page = get_channel_page(uri.to_string(), true, &pool).await?;
    Ok(HttpResponse::Ok().content_type("text/html").body(page))
}

#[get("/channel/{uri}")]
pub async fn channel_handler(
    uri: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let page = get_channel_page(uri.to_string(), false, &pool).await?;
    Ok(HttpResponse::Ok().content_type("text/html").body(page))
}
