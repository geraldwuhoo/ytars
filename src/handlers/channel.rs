use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{ChannelModel, VideoListModel, VideoType},
    util::{_default_video_type, get_cookie_value_bool},
};

#[derive(Debug, Template)]
#[template(path = "channel.html")]
struct ChannelTemplate {
    channel: ChannelModel,
    videos: Vec<VideoListModel>,
    video_type: VideoType,
    show_thumbnails: bool,
    likes_dislikes_on_channel_page: bool,
}

#[derive(Copy, Clone, Debug, Deserialize)]
enum Sort {
    Latest,
    Popular,
    Oldest,
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
            DESC;"#,
                channel_id,
                video_type as VideoType,
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
            DESC;"#,
                channel_id,
                video_type as VideoType,
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
            ASC;"#,
                channel_id,
                video_type as VideoType,
            )
            .fetch_all(pool.as_ref())
            .await?
        }
    };

    let channel = sqlx::query_as!(
        ChannelModel,
        "SELECT * FROM channel WHERE id = $1;",
        channel_id,
    )
    .fetch_one(pool.as_ref())
    .await?;

    let ytchannel = ChannelTemplate {
        channel,
        videos,
        video_type,
        show_thumbnails,
        likes_dislikes_on_channel_page,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(ytchannel.render()?))
}
