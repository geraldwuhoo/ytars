mod channel;
mod errors;
mod feed;
mod file;
mod home;
mod model;
mod scan;
mod video;

use actix_web::{middleware::Logger, web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use clap::Parser;
use scan::scan_handler;
use sqlx::postgres::PgPoolOptions;
use std::sync::{atomic::AtomicBool, Arc};

use crate::channel::{channel_handler, shorts_handler};
use crate::errors::YtarsError;
use crate::feed::feed_handler;
use crate::file::index_handler;
use crate::home::home_handler;
use crate::video::yt_video_handler;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to downloaded YouTube videos directory
    #[arg(long, env, default_value = "/videos")]
    video_path: String,

    // Port to bind to
    #[arg(long, env, default_value_t = 8080)]
    bind_port: u16,

    // Address to bind to
    #[arg(long, env, default_value = "0.0.0.0")]
    bind_address: String,

    // Postgres username
    #[arg(long, env, default_value = "ytars")]
    postgres_username: String,

    // Postgres password
    #[arg(long, env, default_value = "password")]
    postgres_password: String,

    // Postgres host
    #[arg(long, env, default_value = "localhost")]
    postgres_host: String,

    // Postgres DB name
    #[arg(long, env, default_value = "ytars")]
    postgres_db: String,
}

#[actix_web::main]
async fn main() -> Result<(), YtarsError> {
    let args = Args::parse();
    let video_path = args.video_path;
    let scanning = Arc::new(AtomicBool::new(false));

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!(
            "postgres://{}:{}@{}/{}",
            args.postgres_username, args.postgres_password, args.postgres_host, args.postgres_db,
        ))
        .await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(HttpServer::new(move || {
        let generated = generate();
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(video_path.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(scanning.clone()))
            .service(home_handler)
            .service(scan_handler)
            .service(feed_handler)
            .service(channel_handler)
            .service(shorts_handler)
            .service(yt_video_handler)
            .service(index_handler)
            .service(ResourceFiles::new("/static", generated))
    })
    .bind((args.bind_address, args.bind_port))?
    .run()
    .await?)
}
