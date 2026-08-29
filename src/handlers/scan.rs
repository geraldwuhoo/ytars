// TODO: Refactor this horrible file

use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use futures::{stream, StreamExt, TryStreamExt};
use glob::glob;
use image::{imageops::FilterType, ImageFormat, ImageReader};
use log::{debug, info};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU8, Ordering},
        Arc, OnceLock,
    },
};
use time::{macros::format_description, Date, OffsetDateTime};

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoJson, VideoType},
    util::_default_false,
};

/// Outstanding dislike lookups against the remote API.
const HTTP_FETCH_CONCURRENCY: usize = 10;
/// Concurrent dislike writes. Kept at or below the pool size in `main`.
const DB_WRITE_CONCURRENCY: usize = 5;
/// Rows per transaction, so a cold scan costs one commit per batch rather than
/// one per row.
const INSERT_BATCH: usize = 100;

/// Reusing one client keeps its connection pool and TLS setup across scans.
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

/// Number of thumbnails to decode/resize at once. These are CPU-bound and run
/// on the blocking pool, so scale with the machine.
fn thumbnail_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Which half of a scan is running. Kept as a `u8` so it lives in an atomic
/// next to the counters instead of behind a lock.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum ScanStage {
    Videos = 0,
    Dislikes = 1,
}

impl ScanStage {
    fn label(self) -> &'static str {
        match self {
            ScanStage::Videos => "Scanning videos",
            ScanStage::Dislikes => "Fetching likes/dislikes",
        }
    }
}

impl From<u8> for ScanStage {
    fn from(value: u8) -> Self {
        match value {
            1 => ScanStage::Dislikes,
            _ => ScanStage::Videos,
        }
    }
}

/// The scan slot, plus counters for whichever scan holds it. Shared with the
/// `/scan` handler so a page load during a scan can report how far it has got.
/// Only the task that claimed the slot writes the counters, so relaxed
/// ordering is enough: a reader may see a slightly stale count, never a torn
/// one.
#[derive(Debug, Default)]
pub struct ScanState {
    running: AtomicBool,
    stage: AtomicU8,
    started_at: AtomicI64,
    channels_done: AtomicU32,
    channels_total: AtomicU32,
    videos_scanned: AtomicU32,
    videos_added: AtomicU32,
    thumbnails: AtomicU32,
    dislikes_done: AtomicU32,
    dislikes_total: AtomicU32,
}

impl ScanState {
    /// Claims the scan slot and zeroes the counters for the new run, or
    /// returns false if a scan is already running.
    fn begin(&self) -> bool {
        // Claim the slot atomically. The previous check-then-set was racy, and
        // it set the flag inside the spawned task -- which under spawn_local
        // does not run until the handler yields -- so a second request
        // arriving in between would start a duplicate concurrent scan.
        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }

        self.stage.store(ScanStage::Videos as u8, Ordering::Relaxed);
        self.started_at.store(
            OffsetDateTime::now_utc().unix_timestamp(),
            Ordering::Relaxed,
        );
        for counter in [
            &self.channels_done,
            &self.channels_total,
            &self.videos_scanned,
            &self.videos_added,
            &self.thumbnails,
            &self.dislikes_done,
            &self.dislikes_total,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
        true
    }

    /// Progress of the scan in flight, or `None` when nothing is running.
    /// Read as one snapshot so the template cannot see the counters move
    /// underneath it.
    pub fn progress(&self) -> Option<ScanProgress> {
        if !self.running.load(Ordering::Acquire) {
            return None;
        }

        let elapsed =
            OffsetDateTime::now_utc().unix_timestamp() - self.started_at.load(Ordering::Relaxed);
        Some(ScanProgress {
            stage: ScanStage::from(self.stage.load(Ordering::Relaxed)).label(),
            elapsed: format_elapsed(elapsed.max(0)),
            channels_done: self.channels_done.load(Ordering::Relaxed),
            channels_total: self.channels_total.load(Ordering::Relaxed),
            videos_scanned: self.videos_scanned.load(Ordering::Relaxed),
            videos_added: self.videos_added.load(Ordering::Relaxed),
            thumbnails: self.thumbnails.load(Ordering::Relaxed),
            dislikes_done: self.dislikes_done.load(Ordering::Relaxed),
            dislikes_total: self.dislikes_total.load(Ordering::Relaxed),
        })
    }
}

/// A rendered snapshot of `ScanState`, handed to the scan template.
#[derive(Debug)]
pub struct ScanProgress {
    stage: &'static str,
    elapsed: String,
    channels_done: u32,
    channels_total: u32,
    videos_scanned: u32,
    videos_added: u32,
    thumbnails: u32,
    dislikes_done: u32,
    dislikes_total: u32,
}

/// 95 -> "1m 35s". Whole seconds are plenty for a page the user refreshes by
/// hand.
fn format_elapsed(seconds: i64) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Clears the scanning flag on every exit path, including a panic. Without this
/// an early return would leave the flag set and block all later scans.
struct ScanGuard(Arc<ScanState>);

impl Drop for ScanGuard {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Release);
    }
}

#[derive(Debug, Deserialize)]
pub struct ScanParams {
    #[serde(default = "_default_false")]
    overwrite: bool,
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
    progress: Option<ScanProgress>,
}

async fn thumbnail_image(
    width: u32,
    height: u32,
    image_format: Option<ImageFormat>,
    path: &Path,
) -> Result<Vec<u8>, YtarsError> {
    // Decoding, resizing and re-encoding are CPU-bound and never yield. Actix
    // workers each run a single-threaded runtime, so doing this inline starves
    // every other request pinned to the same worker.
    let path = path.to_path_buf();
    web::block(move || {
        let image = ImageReader::open(path)?.with_guessed_format()?.decode()?;
        let image = image.resize_to_fill(width, height, FilterType::Triangle);
        let image_format = image_format.unwrap_or(ImageFormat::WebP);
        let mut image_bytes = Vec::new();
        image.write_to(&mut Cursor::new(&mut image_bytes), image_format)?;
        Ok::<_, YtarsError>(image_bytes)
    })
    .await?
}

async fn get_all_dislikes(pool: &PgPool, state: &ScanState) -> Result<u32, YtarsError> {
    // Filter in the query rather than fetching every row and discarding most
    // of them in Rust on each scan.
    let videos = sqlx::query!(
        r#"SELECT id
        FROM video
        WHERE likes IS NULL AND dislikes IS NULL"#,
    )
    .fetch_all(pool)
    .await?;

    state
        .stage
        .store(ScanStage::Dislikes as u8, Ordering::Relaxed);
    state
        .dislikes_total
        .store(videos.len() as u32, Ordering::Relaxed);

    let mut pull_count: u32 = 0;

    let client = http_client();
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
        .buffer_unordered(HTTP_FETCH_CONCURRENCY)
        .try_for_each_concurrent(DB_WRITE_CONCURRENCY, |likes_dislikes| async move {
            sqlx::query!(
                r#"UPDATE video
                SET likes=$1, dislikes=$2
                WHERE id=$3"#,
                likes_dislikes.likes.unwrap_or_else(|| 0),
                likes_dislikes.dislikes.unwrap_or_else(|| 0),
                likes_dislikes.id,
            )
            .execute(pool)
            .await?;
            state.dislikes_done.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .await?;

    Ok(pull_count)
}

/// A parsed video row waiting to be written. Buffered so inserts can be
/// committed in batches instead of one autocommit (and one WAL flush) per row.
struct PendingVideo {
    id: String,
    title: String,
    filename: String,
    filestem: String,
    upload_date: Date,
    duration_string: String,
    description: Option<String>,
    channel_id: String,
    video_type: VideoType,
    view_count: i64,
}

async fn flush_videos(pool: &PgPool, batch: &mut Vec<PendingVideo>) -> Result<(), YtarsError> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for video in batch.iter() {
        sqlx::query!(
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
            video.filename,
            video.filestem,
            video.upload_date,
            video.duration_string,
            video.description,
            video.channel_id,
            video.video_type as VideoType,
            video.view_count,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    batch.clear();
    Ok(())
}

async fn flush_thumbnails(
    pool: &PgPool,
    batch: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), YtarsError> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for (video_id, thumbnail) in batch.iter() {
        sqlx::query!(
            r#"INSERT INTO video_thumbnail (id, thumbnail)
                VALUES ($1, $2)
                ON CONFLICT (id)
                DO UPDATE
                SET
                    thumbnail=$2"#,
            video_id,
            thumbnail,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    batch.clear();
    Ok(())
}

async fn populate_videos_in_channel(
    path: &Path,
    sanitized_channel: String,
    overwrite: bool,
    pool: &PgPool,
    // filestem -> video id, and the set of video ids that already have a
    // thumbnail. Loaded once per scan so the per-video checks below are
    // in-memory instead of a database round trip each.
    existing_videos: &mut HashMap<String, String>,
    existing_thumbnails: &mut HashSet<String>,
    state: &ScanState,
) -> Result<(u32, u32), YtarsError> {
    let channel_dir = path.join(sanitized_channel);
    let paths: Vec<PathBuf> = web::block(move || {
        Ok::<_, std::io::Error>(
            fs::read_dir(channel_dir)?
                .filter_map(|r| r.ok())
                .map(|r| r.path())
                .filter(|r| {
                    r.is_file()
                        && (r.extension().unwrap_or_default() == "webm"
                            || r.extension().unwrap_or_default() == "mp4")
                })
                .collect(),
        )
    })
    .await??;

    let (mut scan_count, mut all_count) = (0u32, 0u32);
    let mut pending_videos: Vec<PendingVideo> = Vec::new();
    let mut pending_thumbnails: Vec<(String, PathBuf)> = Vec::new();
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
        state.videos_scanned.fetch_add(1, Ordering::Relaxed);

        let known_id = existing_videos.get(&filestem).cloned();

        if overwrite || known_id.is_none() {
            info!("Working on {}", filestem);
            let info_json_path = full_path.with_extension("info.json");
            let jsoncontents = web::block(move || fs::read_to_string(info_json_path)).await??;
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
            let short = (!video.duration_string.contains(':')
                || (video.duration_string.len() == 4 && video.duration_string.as_str() <= "3:00"))
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

            existing_videos.insert(filestem.clone(), video.id.clone());
            pending_videos.push(PendingVideo {
                id: video.id,
                title: video.title,
                filename,
                filestem: filestem.clone(),
                upload_date: date,
                duration_string,
                description,
                channel_id: video.channel_id,
                video_type,
                view_count: video.view_count,
            });
            if pending_videos.len() >= INSERT_BATCH {
                flush_videos(pool, &mut pending_videos).await?;
            }
            scan_count += 1;
            state.videos_added.fetch_add(1, Ordering::Relaxed);
        } else {
            debug!("Video {} exists and overwrite not set, skipping", filestem,);
        }

        if let Some(video_id) = existing_videos.get(&filestem).cloned() {
            if overwrite || !existing_thumbnails.contains(&video_id) {
                pending_thumbnails.push((video_id, full_path));
            }
        }
    }
    flush_videos(pool, &mut pending_videos).await?;

    // Thumbnails are the dominant cost of a cold scan and are independent of
    // one another, so decode/resize several at a time rather than one by one.
    // Work in chunks so the generated bytes stay bounded in memory and each
    // chunk commits as a single transaction.
    let concurrency = thumbnail_concurrency();
    for chunk in pending_thumbnails.chunks(INSERT_BATCH) {
        let mut generated: Vec<(String, Vec<u8>)> = stream::iter(chunk.to_vec())
            .map(|(video_id, full_path)| async move {
                // Only touch the filesystem once we know a thumbnail is needed.
                let webp_path = full_path.with_extension("webp");
                let jpg_path = full_path.with_extension("jpg");
                let thumbnail_path = web::block(move || {
                    if webp_path.exists() {
                        webp_path
                    } else {
                        jpg_path
                    }
                })
                .await?;

                info!("Resizing thumbnail at {}", thumbnail_path.display());
                let resized_thumbnail =
                    thumbnail_image(320, 180, Some(ImageFormat::Jpeg), &thumbnail_path).await?;
                state.thumbnails.fetch_add(1, Ordering::Relaxed);
                Ok::<_, YtarsError>((video_id, resized_thumbnail))
            })
            .buffer_unordered(concurrency)
            .try_collect()
            .await?;

        for (video_id, _) in generated.iter() {
            existing_thumbnails.insert(video_id.clone());
        }
        flush_thumbnails(pool, &mut generated).await?;
    }

    Ok((scan_count, all_count))
}

async fn populate_channel_in_db(
    path: &Path,
    overwrite: bool,
    pool: &PgPool,
    state: &ScanState,
) -> Result<(u32, u32), YtarsError> {
    if overwrite {
        debug!("Overwrite requested, deleting all existing data...");
        sqlx::query!("TRUNCATE TABLE video, channel")
            .execute(pool)
            .await?;
    }

    // Two queries for the whole scan, replacing three round trips per video.
    let mut existing_videos: HashMap<String, String> =
        sqlx::query!("SELECT filestem, id FROM video")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| (row.filestem, row.id))
            .collect();
    let mut existing_thumbnails: HashSet<String> = sqlx::query!("SELECT id FROM video_thumbnail")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect();

    debug!("Populating database...");
    let root_dir = path.to_path_buf();
    let channels: Vec<PathBuf> = web::block(move || {
        Ok::<_, std::io::Error>(
            fs::read_dir(root_dir)?
                .filter_map(|r| r.ok())
                .map(|r| r.path())
                .filter(|r| r.is_dir())
                .collect(),
        )
    })
    .await??;

    state
        .channels_total
        .store(channels.len() as u32, Ordering::Relaxed);

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

        if overwrite || channel.is_none() {
            let json_glob = path
                .join(channel_name)
                .join(format!("{} - Videos *.info.json", channel_name))
                .to_str()
                .ok_or_else(|| YtarsError::Other("Failed to create json glob path".to_string()))?
                .to_string();

            let json_path = web::block(move || {
                glob(&json_glob)?
                    .next()
                    .transpose()
                    .map_err(YtarsError::from)
            })
            .await??
            .ok_or(YtarsError::Other(format!(
                "No results returned for glob {}",
                channel_name
            )))?;
            let thumbnail_path = json_path.with_extension("").with_extension("jpg");

            let json_contents = web::block(move || fs::read_to_string(json_path)).await??;
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
            pool,
            &mut existing_videos,
            &mut existing_thumbnails,
            state,
        )
        .await?;
        scan_count += channel_scan_count;
        all_count += channel_all_count;
        state.channels_done.fetch_add(1, Ordering::Relaxed);
    }

    Ok((scan_count, all_count))
}

pub async fn scan_full(
    video_path: Arc<PathBuf>,
    overwrite: bool,
    pool: PgPool,
    scanning: Arc<ScanState>,
) -> Result<String, YtarsError> {
    if !scanning.begin() {
        return Ok("Already running a scan, please wait until complete".to_string());
    }

    let status = if overwrite {
        "Force scan started"
    } else {
        "Scan started"
    };
    actix_web::rt::spawn({
        // These are all Arcs, either explicitly or internally
        let video_path = Arc::clone(&video_path);
        let pool = pool.clone();
        let scanning = Arc::clone(&scanning);

        async move {
            // Releases the flag on every exit path, including a panic.
            let _guard = ScanGuard(Arc::clone(&scanning));
            // Add all videos and create thumbnails
            match populate_channel_in_db(&video_path, overwrite, &pool, &scanning).await {
                Ok((scan_count, all_count)) => {
                    info!("Finished scan: {} added, {} scanned", scan_count, all_count)
                }
                Err(e) => info!("Error scanning: {}", e),
            };
            // Add all dislikes for videos
            match get_all_dislikes(&pool, &scanning).await {
                Ok(pull_count) => info!("Finished dislikes: {} added", pull_count),
                Err(e) => info!("Error scanning: {}", e),
            }
        }
    });

    Ok(status.to_string())
}

#[get("/scan")]
pub async fn scan_handler(
    params: web::Query<ScanParams>,
    video_path: web::Data<PathBuf>,
    pool: web::Data<PgPool>,
    scanning: web::Data<Arc<ScanState>>,
) -> Result<HttpResponse, YtarsError> {
    let overwrite = params.overwrite;
    let status = scan_full(
        (*video_path).clone(),
        overwrite,
        (**pool).clone(),
        (**scanning).clone(),
    )
    .await?;
    info!("{}", status);

    let scan = ScanTemplate {
        status: &status,
        progress: scanning.progress(),
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(scan.render()?))
}
