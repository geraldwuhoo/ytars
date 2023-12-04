use actix_web::{get, web, HttpResponse, Result};
use askama::Template;
use sqlx::PgPool;

use crate::{errors::YtarsError, model::ChannelModel};

#[derive(Debug, Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    channels: Vec<ChannelModel>,
}

#[get("/")]
pub async fn home_handler(pool: web::Data<PgPool>) -> Result<HttpResponse, YtarsError> {
    let channels = sqlx::query_as!(ChannelModel, "SELECT * FROM channel ORDER BY LOWER(name);")
        .fetch_all(pool.get_ref())
        .await?;

    let home = HomeTemplate { channels };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(home.render()?))
}
