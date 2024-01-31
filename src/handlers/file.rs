use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use log::debug;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, ChannelThumbnailModel, VideoThumbnailModel},
};

#[derive(Deserialize)]
struct Info {
    channel: String,
    filename: String,
}

async fn get_full_path(
    req: HttpRequest,
    info: Info,
    root_path: &PathBuf,
) -> Result<HttpResponse, YtarsError> {
    let not_found = Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body("404 Not Found"));
    debug!("Getting file: {}", info.filename);

    let full_path = match root_path
        .join(&info.channel)
        .join(&info.filename)
        .canonicalize()
    {
        Ok(p) => p,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => return not_found,
            _ => return Err(e.into()),
        },
    };
    debug!(
        "Getting full_path, root_path: {:?}, {:?}",
        full_path, root_path,
    );

    if !full_path.starts_with(root_path.as_path()) {
        return not_found;
    }

    match NamedFile::open_async(full_path).await {
        Ok(file) => Ok(file.into_response(&req)),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => not_found,
            _ => Err(e.into()),
        },
    }
}

#[get("/thumbnails/channel/{id}")]
pub async fn thumbnail_channel_handler(
    req: HttpRequest,
    path: web::Path<String>,
    root_path: web::Data<PathBuf>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let id = path.into_inner();
    debug!("Getting channel thumbnail for {}", id);

    let channel = sqlx::query_as!(
        ChannelThumbnailModel,
        "SELECT * FROM channel_thumbnail WHERE id = $1;",
        id
    )
    .fetch_optional(pool.get_ref())
    .await?;

    if let Some(channel) = channel {
        Ok(HttpResponse::Ok()
            .content_type("image/webp")
            .body(channel.thumbnail))
    } else {
        debug!(
            "No thumbnail for {} found, attempting to return full image",
            id
        );
        let channel = sqlx::query_as!(ChannelModel, "SELECT * FROM channel WHERE id = $1;", id)
            .fetch_one(pool.get_ref())
            .await?;

        get_full_path(
            req,
            Info {
                channel: channel.sanitized_name.clone(),
                filename: format!("{} - Videos [{}].jpg", channel.sanitized_name, channel.id),
            },
            &root_path,
        )
        .await
    }
}

#[get("/thumbnails/video/{id}")]
pub async fn thumbnail_video_handler(
    req: HttpRequest,
    path: web::Path<String>,
    root_path: web::Data<PathBuf>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let id = path.into_inner();
    debug!("Getting video thumbnail for {}", id);

    let channel = sqlx::query_as!(
        VideoThumbnailModel,
        "SELECT * FROM video_thumbnail WHERE id = $1;",
        id
    )
    .fetch_optional(pool.get_ref())
    .await?;

    if let Some(video) = channel {
        Ok(HttpResponse::Ok()
            .content_type("image/jpeg")
            .body(video.thumbnail))
    } else {
        debug!(
            "No thumbnail for {} found, attempting to return full image",
            id
        );
        let data = sqlx::query!(
            "SELECT
                video.filestem AS filestem,
                channel.sanitized_name AS channel_sanitized_name
            FROM video
            INNER JOIN channel on video.channel_id = channel.id
            WHERE video.id = $1;",
            id,
        )
        .fetch_one(pool.get_ref())
        .await?;

        get_full_path(
            req,
            Info {
                channel: data.channel_sanitized_name,
                filename: format!("{}.webp", data.filestem),
            },
            &root_path,
        )
        .await
    }
}

#[get("/files/{channel}/{filename}")]
pub async fn index_handler(
    req: HttpRequest,
    info: web::Path<Info>,
    root_path: web::Data<PathBuf>,
) -> Result<HttpResponse, YtarsError> {
    get_full_path(req, info.into_inner(), &root_path).await
}
