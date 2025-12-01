mod handlers;
mod structures;

use actix_web::{middleware::Logger, rt::time::sleep, web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use clap::Parser;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use crate::{
    handlers::{
        channel::channel_handler,
        feed::feed_handler,
        file::{index_handler, thumbnail_channel_handler, thumbnail_video_handler},
        home::home_handler,
        preferences::{preferences_get_handler, preferences_post_handler},
        scan::{scan_full, scan_handler},
        search::search_handler,
        video::yt_video_handler,
    },
    structures::errors::YtarsError,
};

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to downloaded YouTube videos directory
    #[arg(long, env, default_value = "/videos")]
    video_path: String,

    /// Interval to scan for new videos (in seconds)
    #[arg(long, env, default_value_t = 3600)]
    scan_interval: u64,

    /// Port to bind to
    #[arg(long, env, default_value_t = 8080)]
    bind_port: u16,

    /// Address to bind to
    #[arg(long, env, default_value = "0.0.0.0")]
    bind_address: String,

    /// Postgres username
    #[arg(long, env, default_value = "ytars")]
    postgres_username: String,

    /// Postgres password
    #[arg(long, env, default_value = "password")]
    postgres_password: String,

    /// Postgres host
    #[arg(long, env, default_value = "localhost")]
    postgres_host: String,

    /// Postgres DB name
    #[arg(long, env, default_value = "ytars")]
    postgres_db: String,
}

#[actix_web::main]
async fn main() -> Result<(), YtarsError> {
    let args = Args::parse();
    let video_path = Path::new(&args.video_path).canonicalize()?;
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

    actix_web::rt::spawn({
        let video_path = Arc::new(video_path.clone());
        let pool = pool.clone();
        let scanning = Arc::clone(&scanning);

        async move {
            loop {
                info!("Background scan: Scanning");
                match scan_full(
                    Arc::clone(&video_path),
                    false,
                    pool.clone(),
                    Arc::clone(&scanning),
                )
                .await
                {
                    Ok(status) => info!("Background scan: {}", status),
                    Err(e) => error!("Background scan: Failed to scan: {}", e),
                }

                info!("Background scan: Sleeping");
                sleep(Duration::from_secs(args.scan_interval)).await;
            }
        }
    });

    Ok(HttpServer::new(move || {
        let generated = generate();
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(video_path.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Arc::clone(&scanning)))
            .service(home_handler)
            .service(preferences_get_handler)
            .service(preferences_post_handler)
            .service(scan_handler)
            .service(search_handler)
            .service(feed_handler)
            .service(channel_handler)
            .service(yt_video_handler)
            .service(thumbnail_channel_handler)
            .service(thumbnail_video_handler)
            .service(index_handler)
            .service(ResourceFiles::new("/static", generated))
    })
    .bind((args.bind_address, args.bind_port))?
    .run()
    .await?)
}
