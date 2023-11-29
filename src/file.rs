use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use log::debug;
use serde::Deserialize;

use crate::errors::YtarsError;

#[derive(Deserialize)]
struct Info {
    channel: String,
    filename: String,
}

#[get("/files/{channel}/{filename}")]
pub async fn index_handler(
    req: HttpRequest,
    info: web::Path<Info>,
    root_path: web::Data<String>,
) -> Result<HttpResponse, YtarsError> {
    let info = info.into_inner();
    debug!("Getting file: {}", info.filename);

    let root_path: &str = &root_path;
    let full_path = PathBuf::from(root_path)
        .as_path()
        .join(info.channel)
        .join(info.filename);

    match NamedFile::open_async(full_path).await {
        Ok(file) => Ok(file.into_response(&req)),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body("404 Not Found")),
            _ => Err(e.into()),
        },
    }
}
