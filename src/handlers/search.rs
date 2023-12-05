use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use serde::Deserialize;
use sqlx::PgPool;

use crate::structures::{
    errors::YtarsError,
    model::{VideoChannelJoinModel, VideoType},
};

#[derive(Debug, Template)]
#[template(path = "search.html")]
struct SearchTemplate<'a> {
    videos: Vec<VideoChannelJoinModel>,
    query: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    query: String,
}

#[get("/search")]
pub async fn search_handler(
    params: web::Query<SearchParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let query = &params.query;
    println!("{}", query);
    let videos = sqlx::query_as!(
        VideoChannelJoinModel,
        r#"SELECT
            video.id,
            title,
            upload_date,
            duration_string,
            channel.id AS channel_id,
            channel.name,
            video_type AS "video_type: VideoType"
        FROM video
        INNER JOIN channel ON video.channel_id = channel.id
        WHERE document @@ plainto_tsquery($1)
        ORDER BY ts_rank(document, plainto_tsquery($1)) DESC
        LIMIT 100;"#,
        query,
    )
    .fetch_all(pool.as_ref())
    .await?;

    let search = SearchTemplate { videos, query };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(search.render()?))
}
