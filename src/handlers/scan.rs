use actix_web::{get, web, HttpResponse, Result};
use glob::glob;
use log::{debug, info};
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use time::{macros::format_description, Date};

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoJson, VideoType},
};

#[derive(Debug, Deserialize)]
pub struct ScanParams {
    #[serde(default = "_default_false")]
    overwrite: bool,
}

const fn _default_false() -> bool {
    false
}

async fn populate_channel(
    path: &Path,
    sanitized_channel: String,
    overwrite: bool,
    pool: &PgPool,
) -> Result<(), YtarsError> {
    let paths = fs::read_dir(path.join(sanitized_channel))?
        .filter_map(|r| r.ok())
        .map(|r| r.path())
        .filter(|r| {
            r.is_file()
                && (r.extension().unwrap_or_default() == "webm"
                    || r.extension().unwrap_or_default() == "mp4")
        });

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

        if !overwrite {
            let video = sqlx::query!("SELECT filestem FROM video WHERE filestem = $1;", filestem)
                .fetch_optional(pool)
                .await?;
            if video.is_some() {
                debug!("Skipping {}", filestem);
                continue;
            }
        }

        debug!("Working on {}", filestem);
        let jsoncontents = fs::read_to_string(full_path.clone().with_extension("info.json"))?;
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
            r#"INSERT INTO video (id, title, filename, filestem, upload_date, duration_string, description, channel_id, video_type)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id)
                DO NOTHING"#,
            video.id,
            video.title,
            filename,
            filestem,
            date,
            duration_string,
            description,
            video.channel_id,
            video_type as VideoType,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn populate(path: &PathBuf, overwrite: bool, pool: &PgPool) -> Result<(), YtarsError> {
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

        let mut json_paths = glob(
            path.join(channel_name)
                .join(format!("{} - Videos *.info.json", channel_name))
                .to_str()
                .ok_or_else(|| YtarsError::Other("Failed to create json glob path".to_string()))?,
        )?;

        let json_path = json_paths.next().ok_or(YtarsError::Other(format!(
            "No results returned for glob {}",
            channel_name
        )))??;
        let json_contents = fs::read_to_string(json_path)?;
        let yt_channel = serde_json::from_str::<ChannelModel>(&json_contents)?;

        sqlx::query_as!(
            ChannelModel,
            r#"INSERT INTO channel (id, name, sanitized_name, description)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (id)
                DO NOTHING"#,
            yt_channel.id,
            yt_channel.name,
            channel_name,
            yt_channel.description,
        )
        .execute(pool)
        .await?;

        populate_channel(path, channel_name.to_string(), overwrite, pool).await?;
    }

    debug!("Done populating postgres with video catalog");

    Ok(())
}

#[get("/scan")]
pub async fn scan_handler(
    params: web::Query<ScanParams>,
    video_path: web::Data<PathBuf>,
    pool: web::Data<PgPool>,
    scanning: web::Data<Arc<AtomicBool>>,
) -> HttpResponse {
    let overwrite = params.overwrite;
    let status;
    if scanning.load(Ordering::Acquire) {
        status = "Already running a scan, please wait until complete";
        info!("{}", status);
    } else {
        status = if overwrite {
            "Force scan started"
        } else {
            "Scan started"
        };
        actix_web::rt::spawn(async move {
            info!("{}{}", status, if overwrite { " (overwrite)" } else { "" });
            scanning.store(true, Ordering::Release);
            if let Err(e) = populate(&video_path, overwrite, &pool).await {
                info!("Error scanning: {}", e);
            } else {
                info!("Finished scan");
            }
            scanning.store(false, Ordering::Release);
        });
    }

    HttpResponse::Ok().body(status)
}
