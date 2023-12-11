use thiserror::Error;

#[derive(Error, Debug)]
pub enum YtarsError {
    #[error("IO error\n{0}")]
    ReadError(#[from] std::io::Error),

    #[error("askama templating error\n{0}")]
    AskamaError(#[from] askama::Error),

    #[error("json error\n{0}")]
    JsonError(#[from] serde_json::Error),

    #[error("sqlx error\n{0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("sqlx migrate error\n{0}")]
    SqlxMigrateError(#[from] sqlx::migrate::MigrateError),

    #[error("pattern error\n{0}")]
    PatternError(#[from] glob::PatternError),

    #[error("glob error\n{0}")]
    GlobError(#[from] glob::GlobError),

    #[error("time parse error\n{0}")]
    TimeParseError(#[from] time::error::Parse),

    #[error("time invalid format error\n{0}")]
    TimeInvalidFormatError(#[from] time::error::InvalidFormatDescription),

    #[error("time format error\n{0}")]
    TimeFormatError(#[from] time::error::Format),

    #[error("reqwest error\n{0}")]
    ParseError(#[from] reqwest::Error),

    #[error("other error\n{0}")]
    Other(String),
}

impl actix_web::error::ResponseError for YtarsError {}
