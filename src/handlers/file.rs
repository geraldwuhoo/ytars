use std::path::PathBuf;

use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use log::debug;
use serde::Deserialize;

use crate::structures::errors::YtarsError;

#[derive(Deserialize)]
struct Info {
    channel: String,
    filename: String,
}

#[get("/files/{channel}/{filename}")]
pub async fn index_handler(
    req: HttpRequest,
    info: web::Path<Info>,
    root_path: web::Data<PathBuf>,
) -> Result<HttpResponse, YtarsError> {
    let not_found = Ok(HttpResponse::NotFound()
        .content_type("text/html")
        .body("404 Not Found"));
    debug!("Getting file: {}", info.filename);

    let full_path = match root_path
        .join(&info.channel)
        .join(&info.filename)
        .canonicalize()
    {
        Ok(p) => p,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => return not_found,
            _ => return Err(e.into()),
        },
    };
    debug!(
        "Getting full_path, root_path: {:?}, {:?}",
        full_path, root_path,
    );

    if !full_path.starts_with(root_path.as_path()) {
        return not_found;
    }

    match NamedFile::open_async(full_path).await {
        Ok(file) => Ok(file.into_response(&req)),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => not_found,
            _ => Err(e.into()),
        },
    }
}
