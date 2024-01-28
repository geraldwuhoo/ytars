use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use askama::Template;
use sqlx::PgPool;

use crate::structures::{errors::YtarsError, model::ChannelModel, util::get_cookie_value_bool};

#[derive(Debug, Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    channels: Vec<ChannelModel>,
    show_avatars: bool,
}

#[get("/")]
pub async fn home_handler(
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, YtarsError> {
    let show_avatars = get_cookie_value_bool(&req, "channel_avatars_on_homepage")?;
    let channels = sqlx::query_as!(ChannelModel, "SELECT * FROM channel ORDER BY LOWER(name);")
        .fetch_all(pool.get_ref())
        .await?;

    let home = HomeTemplate {
        channels,
        show_avatars,
    };
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(home.render()?))
}
