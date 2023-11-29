use actix_web::{
    get,
    http::header::{self, HeaderValue},
    web, HttpResponse, Result,
};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;
use time::format_description;

use crate::{
    errors::YtarsError,
    model::{ChannelModel, VideoModel},
};

#[derive(Debug, Template)]
#[template(path = "video.html")]
struct VideoTemplate {
    video: VideoModel,
    channel: ChannelModel,
    upload_date: String,
}

#[derive(Debug, Deserialize)]
pub struct VideoParams {
    v: Option<String>,
}

#[get("/watch")]
pub async fn yt_video_handler(
    params: web::Query<VideoParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let video_id;
    if let Some(id) = &params.v {
        video_id = id;
    } else {
        return Ok(HttpResponse::PermanentRedirect()
            .append_header((header::LOCATION, HeaderValue::from_static("/")))
            .finish());
    }

    let video = sqlx::query_as!(VideoModel, "SELECT * FROM video WHERE id = $1;", video_id,)
        .fetch_one(pool.get_ref())
        .await?;
    let channel = sqlx::query_as!(
        ChannelModel,
        "SELECT * FROM channel WHERE id = $1;",
        video.channel_id,
    )
    .fetch_one(pool.get_ref())
    .await?;

    let format = format_description::parse("[month repr:long] [day padding:none], [year]")?;
    let upload_date = video.upload_date.format(&format)?;

    let vid = VideoTemplate {
        video,
        channel,
        upload_date,
    };
    let vid = vid.render()?;

    Ok(HttpResponse::Ok().content_type("text/html").body(vid))
}
