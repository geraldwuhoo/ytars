// TODO: Refactor this horrible file

use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use futures::{stream, StreamExt, TryStreamExt};
use glob::glob;
use image::{imageops::FilterType, io::Reader, ImageOutputFormat};
use log::{debug, info};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use time::{macros::format_description, Date};

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoJson, VideoLikesDislikes, VideoType},
    util::_default_false,
};

#[derive(Debug, Deserialize)]
pub struct ScanParams {
    #[serde(default = "_default_false")]
    overwrite: bool,
    #[serde(default = "_default_false")]
    dislikes: bool,
    #[serde(default = "_default_false")]
    thumbnails: bool,
}

#[derive(Debug, Deserialize)]
struct LikesDislikes {
    #[serde(skip_deserializing)]
    id: String,
    likes: Option<i32>,
    dislikes: Option<i32>,
}

#[derive(Debug, Template)]
#[template(path = "scan.html")]
struct ScanTemplate<'a> {
    status: &'a str,
}

async fn thumbnail_image(
    width: u32,
    height: u32,
    image_output_format: Option<ImageOutputFormat>,
    path: &PathBuf,
) -> Result<Vec<u8>, YtarsError> {
    let image = Reader::open(path)?.with_guessed_format()?.decode()?;
    let image = image.resize_to_fill(width, height, FilterType::Triangle);
    let image_output_format = image_output_format.unwrap_or(ImageOutputFormat::WebP);
    let mut image_bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut image_bytes), image_output_format)?;
    Ok(image_bytes)
}

async fn get_all_dislikes(pool: &PgPool) -> Result<u32, YtarsError> {
    let videos = sqlx::query_as!(
        VideoLikesDislikes,
        r#"SELECT
            id,
            likes,
            dislikes
        FROM video"#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter(|video| video.likes.is_none() && video.dislikes.is_none());

    let mut pull_count: u32 = 0;

    let client = Client::new();
    stream::iter(videos)
        .map(|video| {
            pull_count += 1;
            info!("Getting dislikes for {} ({})", video.id, pull_count);
            let client = &client;
            async move {
                let url = format!("https://ryd-proxy.kavin.rocks/votes/{}", video.id);
                let response = client.get(&url).send().await?;
                let mut rs = response.json::<LikesDislikes>().await?;
                rs.id = video.id;
                Ok::<LikesDislikes, YtarsError>(rs)
            }
        })
        .buffer_unordered(10)
        .try_for_each(|likes_dislikes| async move {
            sqlx::query_as!(
                VideoLikesDislikes,
                r#"UPDATE video
                SET likes=$1, dislikes=$2
                WHERE id=$3"#,
                likes_dislikes.likes.unwrap_or_else(|| 0),
                likes_dislikes.dislikes.unwrap_or_else(|| 0),
                likes_dislikes.id,
            )
            .execute(pool)
            .await?;
            Ok(())
        })
        .await?;

    Ok(pull_count)
}

async fn populate_videos_in_channel(
    path: &Path,
    sanitized_channel: String,
    overwrite: bool,
    regenerate_thumbnails: bool,
    pool: &PgPool,
) -> Result<(u32, u32), YtarsError> {
    let paths = fs::read_dir(path.join(sanitized_channel))?
        .filter_map(|r| r.ok())
        .map(|r| r.path())
        .filter(|r| {
            r.is_file()
                && (r.extension().unwrap_or_default() == "webm"
                    || r.extension().unwrap_or_default() == "mp4")
        });

    let (mut scan_count, mut all_count) = (0u32, 0u32);
    for full_path in paths {
        let filename = full_path
            .file_name()
            .ok_or_else(|| YtarsError::Other(format!("Failed to find file {:?}", full_path)))?
            .to_str()
            .ok_or_else(|| {
                YtarsError::Other(format!("Failed to convert to str file {:?}", full_path))
            })?
            .to_string();
        let filestem = full_path
            .file_stem()
            .ok_or_else(|| YtarsError::Other(format!("Failed to find file {:?}", full_path)))?
            .to_str()
            .ok_or_else(|| {
                YtarsError::Other(format!("Failed to convert to str file {:?}", full_path))
            })?
            .to_string();
        all_count += 1;

        let video = sqlx::query!("SELECT id FROM video WHERE filestem = $1;", filestem)
            .fetch_optional(pool)
            .await?;

        if overwrite || video.is_none() {
            info!("Working on {}", filestem);
            let jsoncontents = fs::read_to_string(full_path.with_extension("info.json"))?;
            let video: VideoJson = serde_json::from_str(&jsoncontents)?;
            let duration_string = if video.duration_string.contains(':') {
                video.duration_string.clone()
            } else {
                format!("0:{:0>2}", video.duration_string)
            };
            let description = video
                .description
                .as_ref()
                .map(|description| description.replace('\u{0000}', ""));
            let short = (!video.duration_string.contains(':') || video.duration_string == "1:00")
                && video.aspect_ratio < 1.0;
            let video_type = if short {
                VideoType::Short
            } else if video.was_live {
                VideoType::Stream
            } else {
                VideoType::Video
            };
            let format = format_description!("[year][month][day]");
            let date = Date::parse(&video.upload_date, &format)?;

            sqlx::query_as!(
                VideoModel,
                r#"INSERT INTO video (
                    id,
                    title,
                    filename,
                    filestem,
                    upload_date,
                    duration_string,
                    description,
                    channel_id,
                    video_type,
                    view_count
                )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT (id)
                    DO UPDATE
                    SET
                        title=$2,
                        filename=$3,
                        filestem=$4,
                        upload_date=$5,
                        duration_string=$6,
                        description=$7,
                        channel_id=$8,
                        video_type=$9,
                        view_count=$10"#,
                video.id,
                video.title,
                filename,
                filestem,
                date,
                duration_string,
                description,
                video.channel_id,
                video_type as VideoType,
                video.view_count,
            )
            .execute(pool)
            .await?;

            scan_count += 1;
        } else {
            debug!("Video {} exists and overwrite not set, skipping", filestem,);
        }

        if regenerate_thumbnails {
            if let Some(video) = sqlx::query!("SELECT id FROM video WHERE filestem = $1;", filestem)
                .fetch_optional(pool)
                .await?
            {
                let thumbnail_path = full_path.with_extension("webp");
                let thumbnail_path = if thumbnail_path.exists() {
                    thumbnail_path
                } else {
                    full_path.with_extension("jpg")
                };
                let video_thumbnail =
                    sqlx::query!("SELECT id FROM video_thumbnail WHERE id = $1;", video.id)
                        .fetch_optional(pool)
                        .await?;

                if overwrite || video_thumbnail.is_none() {
                    info!("Resizing thumbnail at {}", thumbnail_path.display());
                    let resized_thumbnail = thumbnail_image(
                        320,
                        180,
                        Some(ImageOutputFormat::Jpeg(80)),
                        &thumbnail_path,
                    )
                    .await?;
                    sqlx::query_as!(
                        VideoThumbnailModel,
                        r#"INSERT INTO video_thumbnail (id, thumbnail)
                        VALUES ($1, $2)
                        ON CONFLICT (id)
                        DO UPDATE
                        SET
                            thumbnail=$2"#,
                        video.id,
                        resized_thumbnail,
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok((scan_count, all_count))
}

async fn populate_channel_in_db(
    path: &PathBuf,
    overwrite: bool,
    regenerate_thumbnails: bool,
    pool: &PgPool,
) -> Result<(u32, u32), YtarsError> {
    if overwrite {
        debug!("Overwrite requested, deleting all existing data...");
        sqlx::query!("TRUNCATE TABLE video, channel")
            .execute(pool)
            .await?;
    }

    debug!("Populating database...");
    let channels = fs::read_dir(path)?
        .filter_map(|r| r.ok())
        .map(|r| r.path())
        .filter(|r| r.is_dir());

    let (mut scan_count, mut all_count) = (0u32, 0u32);
    for channel_path in channels {
        let channel_name = channel_path
            .file_name()
            .ok_or_else(|| {
                YtarsError::Other(format!(
                    "Failed to find file for channel {:?}",
                    channel_path
                ))
            })?
            .to_str()
            .ok_or_else(|| {
                YtarsError::Other(format!(
                    "Failed to convert to str file for channel {:?}",
                    channel_path
                ))
            })?;
        debug!("Working on {}", channel_name);
        let channel = sqlx::query!(
            "SELECT sanitized_name FROM channel WHERE sanitized_name = $1;",
            channel_name
        )
        .fetch_optional(pool)
        .await?;

        if overwrite || regenerate_thumbnails || channel.is_none() {
            let mut json_paths = glob(
                path.join(channel_name)
                    .join(format!("{} - Videos *.info.json", channel_name))
                    .to_str()
                    .ok_or_else(|| {
                        YtarsError::Other("Failed to create json glob path".to_string())
                    })?,
            )?;

            let json_path = json_paths.next().ok_or(YtarsError::Other(format!(
                "No results returned for glob {}",
                channel_name
            )))??;
            let thumbnail_path = json_path.with_extension("").with_extension("jpg");

            let json_contents = fs::read_to_string(json_path)?;
            let yt_channel = serde_json::from_str::<ChannelModel>(&json_contents)?;

            sqlx::query_as!(
                ChannelFullModel,
                r#"INSERT INTO channel (id, name, sanitized_name, description, channel_follower_count)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (id)
                    DO UPDATE
                    SET
                        name=$2,
                        sanitized_name=$3,
                        description=$4,
                        channel_follower_count=$5"#,
                yt_channel.id,
                yt_channel.name,
                channel_name,
                yt_channel.description,
                yt_channel.channel_follower_count,
            )
            .execute(pool)
            .await?;

            if regenerate_thumbnails {
                info!("Resizing thumbnail at {}", thumbnail_path.display());
                let channel_thumbnail = sqlx::query!(
                    "SELECT id FROM channel_thumbnail WHERE id=$1",
                    yt_channel.id,
                )
                .fetch_optional(pool)
                .await?;

                if overwrite || channel_thumbnail.is_none() {
                    let resized_thumbnail = thumbnail_image(50, 50, None, &thumbnail_path).await?;
                    sqlx::query_as!(
                        ChannelThumbnailModel,
                        r#"INSERT INTO channel_thumbnail (id, thumbnail)
                            VALUES ($1, $2)
                            ON CONFLICT (id)
                            DO UPDATE
                            SET
                                thumbnail=$2"#,
                        yt_channel.id,
                        resized_thumbnail,
                    )
                    .execute(pool)
                    .await?;
                }
            }
        } else {
            debug!(
                "Channel {} exists and overwrite not set, skipping",
                channel_name,
            );
        }

        let (channel_scan_count, channel_all_count) = populate_videos_in_channel(
            path,
            channel_name.to_string(),
            overwrite,
            regenerate_thumbnails,
            pool,
        )
        .await?;
        scan_count += channel_scan_count;
        all_count += channel_all_count;
    }

    Ok((scan_count, all_count))
}

#[get("/scan")]
pub async fn scan_handler(
    params: web::Query<ScanParams>,
    video_path: web::Data<PathBuf>,
    pool: web::Data<PgPool>,
    scanning: web::Data<Arc<AtomicBool>>,
) -> Result<HttpResponse, YtarsError> {
    let overwrite = params.overwrite;
    let status;
    if scanning.load(Ordering::Acquire) {
        status = "Already running a scan, please wait until complete";
    } else {
        status = if overwrite {
            "Force scan started"
        } else {
            "Scan started"
        };
        actix_web::rt::spawn(async move {
            scanning.store(true, Ordering::Release);
            if params.dislikes {
                match get_all_dislikes(&pool).await {
                    Ok(pull_count) => {
                        info!("Finished dislikes: {} added", pull_count)
                    }
                    Err(e) => info!("Error scanning: {}", e),
                }
            } else {
                match populate_channel_in_db(&video_path, overwrite, params.thumbnails, &pool).await
                {
                    Ok((scan_count, all_count)) => {
                        info!("Finished scan: {} added, {} scanned", scan_count, all_count)
                    }
                    Err(e) => info!("Error scanning: {}", e),
                };
            }
            scanning.store(false, Ordering::Release);
        });
    }
    info!("{}", status);

    let scan = ScanTemplate { status };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(scan.render()?))
}
