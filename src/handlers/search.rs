use actix_web::{
    get,
    http::header::{self, HeaderValue},
    web, HttpResponse, Result,
};
use askama::Template;
use log::debug;
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
    query: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    query: Option<String>,
}

#[get("/search")]
pub async fn search_handler(
    params: web::Query<SearchParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let query = params.query.as_deref();
    let videos = if let Some(query_str) = query {
        if query_str.is_empty() {
            debug!("Empty query string, redirecting to no query string URL");
            return Ok(HttpResponse::PermanentRedirect()
                .append_header((header::LOCATION, HeaderValue::from_static("/search")))
                .finish());
        }
        debug!("Query string {} detected, querying database", query_str);
        sqlx::query_as!(
            VideoChannelJoinModel,
            r#"SELECT
                video.id,
                title,
                upload_date,
                duration_string,
                channel.id AS channel_id,
                channel.name,
                video_type AS "video_type: VideoType",
                view_count,
                channel.sanitized_name AS channel_sanitized_name,
                filestem
            FROM video
            INNER JOIN channel ON video.channel_id = channel.id
            WHERE document @@ plainto_tsquery($1)
            ORDER BY ts_rank(document, plainto_tsquery($1)) DESC
            LIMIT 100;"#,
            query_str,
        )
        .fetch_all(pool.as_ref())
        .await?
    } else {
        debug!("No query string, skipping database query");
        Vec::new()
    };

    let search = SearchTemplate { videos, query };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(search.render()?))
}
