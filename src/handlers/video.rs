use actix_web::{
    get,
    http::header::{self, HeaderValue},
    web, HttpResponse, Result,
};
use askama::Template;
use log::debug;
use serde::Deserialize;
use sqlx::PgPool;
use time::format_description;

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoModel, VideoType},
    util::_default_false,
};

#[derive(Debug, Template)]
#[template(path = "video.html")]
struct VideoTemplate<'a> {
    video: VideoModel,
    channel: ChannelModel,
    upload_date: &'a str,
    feed: bool,
}

#[derive(Debug, Deserialize)]
pub struct VideoParams {
    v: Option<String>,
    #[serde(default = "_default_false")]
    feed: bool,
}

#[get("/watch")]
pub async fn yt_video_handler(
    params: web::Query<VideoParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let video_id = if let Some(id) = &params.v {
        id
    } else {
        return Ok(HttpResponse::PermanentRedirect()
            .append_header((header::LOCATION, HeaderValue::from_static("/")))
            .finish());
    };

    let video = match sqlx::query_as!(
        VideoModel,
        r#"SELECT
            id,
            title,
            filename,
            filestem,
            upload_date,
            duration_string,
            description,
            channel_id,
            video_type AS "video_type: VideoType",
            view_count,
            likes,
            dislikes
        FROM video
        WHERE id = $1;"#,
        video_id
    )
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => match e {
            sqlx::error::Error::RowNotFound => {
                debug!("Couldn't find video {}", video_id);
                return Ok(HttpResponse::NotFound()
                    .content_type("text/html")
                    .body("404 Not Found"));
            }
            x => return Err(x.into()),
        },
    };

    let channel = sqlx::query_as!(
        ChannelModel,
        "SELECT * FROM channel WHERE id = $1;",
        video.channel_id,
    )
    .fetch_one(pool.get_ref())
    .await?;

    let format = format_description::parse("[month repr:long] [day padding:none], [year]")?;
    let upload_date = &video.upload_date.format(&format)?;

    let vid = VideoTemplate {
        video,
        channel,
        upload_date,
        feed: params.feed,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(vid.render()?))
}
