use core::fmt;

use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use askama::Template;
use log::debug;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoListModel, VideoType},
    util::{_default_page, _default_video_type, get_cookie_value_bool, get_cookie_value_i64},
};

#[derive(Debug, Template)]
#[template(path = "channel.html")]
struct ChannelTemplate {
    channel: ChannelModel,
    videos: Vec<VideoListModel>,
    video_count: i64,
    video_type: VideoType,
    show_thumbnails: bool,
    likes_dislikes_on_channel_page: bool,
    page: i64,
    page_size: i64,
    sort_type: Sort,
}

#[derive(Copy, Clone, Debug, Deserialize)]
enum Sort {
    Latest,
    Popular,
    Oldest,
    Longest,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

const fn _default_sort_type() -> Sort {
    Sort::Latest
}

#[derive(Debug, Deserialize)]
pub struct ChannelParams {
    #[serde(default = "_default_video_type")]
    video_type: VideoType,
    #[serde(default = "_default_sort_type")]
    sort: Sort,
    #[serde(default = "_default_page")]
    page: i64,
}

#[get("/channel/{uri}")]
pub async fn channel_handler(
    req: HttpRequest,
    params: web::Query<ChannelParams>,
    uri: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let show_thumbnails = get_cookie_value_bool(&req, "thumbnails_for_all_videos")?;
    let likes_dislikes_on_channel_page =
        get_cookie_value_bool(&req, "likes/dislikes_on_channel_page")?;
    let channel_id = uri.to_string();
    let video_type = params.video_type;
    let page_size = get_cookie_value_i64(&req, "videos_per_page")?;
    let page = params.page;

    debug!(
        "Getting page {} size {} for channel {}",
        page, page_size, channel_id,
    );

    let videos = match params.sort {
        Sort::Latest => {
            sqlx::query_as!(
                VideoListModel,
                r#"SELECT
                    id,
                    title,
                    upload_date,
                    duration_string,
                    channel_id,
                    video_type AS "video_type: VideoType",
                    view_count,
                    filestem,
                    likes,
                    dislikes
                FROM video
                WHERE channel_id = $1 AND video_type = $2
                ORDER BY upload_date
                DESC
                OFFSET $3
                LIMIT $4;"#,
                channel_id,
                video_type as VideoType,
                page * page_size,
                page_size,
            )
            .fetch_all(pool.as_ref())
            .await?
        }
        Sort::Popular => {
            sqlx::query_as!(
                VideoListModel,
                r#"SELECT
                    id,
                    title,
                    upload_date,
                    duration_string,
                    channel_id,
                    video_type AS "video_type: VideoType",
                    view_count,
                    filestem,
                    likes,
                    dislikes
                FROM video
                WHERE channel_id = $1 AND video_type = $2
                ORDER BY view_count
                DESC
                OFFSET $3
                LIMIT $4;"#,
                channel_id,
                video_type as VideoType,
                page * page_size,
                page_size,
            )
            .fetch_all(pool.as_ref())
            .await?
        }
        Sort::Oldest => {
            sqlx::query_as!(
                VideoListModel,
                r#"SELECT
                    id,
                    title,
                    upload_date,
                    duration_string,
                    channel_id,
                    video_type AS "video_type: VideoType",
                    view_count,
                    filestem,
                    likes,
                    dislikes
                FROM video
                WHERE channel_id = $1 AND video_type = $2
                ORDER BY upload_date
                ASC
                OFFSET $3
                LIMIT $4;"#,
                channel_id,
                video_type as VideoType,
                page * page_size,
                page_size,
            )
            .fetch_all(pool.as_ref())
            .await?
        }
        Sort::Longest => {
            sqlx::query_as!(
                VideoListModel,
                r#"SELECT
                    id,
                    title,
                    upload_date,
                    duration_string,
                    channel_id,
                    video_type AS "video_type: VideoType",
                    view_count,
                    filestem,
                    likes,
                    dislikes
                FROM video
                WHERE channel_id = $1 AND video_type = $2
                ORDER BY
                    CHAR_LENGTH(duration_string) DESC,
                    duration_string DESC
                OFFSET $3
                LIMIT $4;"#,
                channel_id,
                video_type as VideoType,
                page * page_size,
                page_size,
            )
            .fetch_all(pool.as_ref())
            .await?
        }
    };

    let video_count = sqlx::query!(
        "SELECT COUNT(id) AS count FROM video WHERE channel_id = $1 AND video_type = $2",
        channel_id,
        video_type as VideoType,
    )
    .fetch_one(pool.as_ref())
    .await?
    .count
    .unwrap_or_default();

    let channel = sqlx::query_as!(
        ChannelModel,
        r#"SELECT id, name, sanitized_name, description, channel_follower_count
            FROM channel WHERE id = $1;"#,
        channel_id,
    )
    .fetch_one(pool.as_ref())
    .await?;

    let ytchannel = ChannelTemplate {
        channel,
        videos,
        video_count,
        video_type,
        show_thumbnails,
        likes_dislikes_on_channel_page,
        page,
        page_size,
        sort_type: params.sort,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(ytchannel.render()?))
}
