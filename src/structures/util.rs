use core::fmt;
use std::collections::HashMap;

use actix_web::HttpRequest;
use lazy_static::lazy_static;
use log::debug;

use super::{errors::YtarsError, model::VideoType};

pub const fn _default_true() -> bool {
    true
}

pub const fn _default_false() -> bool {
    false
}

pub const fn _default_count() -> i64 {
    100
}

pub const fn _default_video_type() -> VideoType {
    VideoType::Video
}

fn truncate(num: f64) -> f64 {
    debug!("got num to truncate: {}", num);
    let num_integ = num.trunc().to_string();
    let num_fract = num.fract().to_string();
    let significant_digits = 3;
    debug!("integ: {}, fract: {}", num_integ, num_fract);

    if (num_integ.len() < 3 && num_fract == "0") || num_integ.len() + num_fract.len() < 5 {
        num
    } else if num_integ.len() == significant_digits {
        num.trunc()
    } else {
        let decimals = significant_digits - num_integ.len() + 2_usize;
        let combined_number: f64 = (num_integ + "." + &num_fract[2..decimals]).parse().unwrap();
        combined_number
    }
}

pub fn follower_count_to_string(count: &i32) -> String {
    debug!("follower count: {}", count);
    if *count < 100 {
        count.to_string()
    } else if count / 1_000_000 > 0 {
        format!("{}M", truncate(*count as f64 / 1_000_000.0))
    } else if count / 1_000 > 0 {
        format!("{}K", truncate(*count as f64 / 1_000.0))
    } else {
        format!("{}", truncate(*count as f64))
    }
}

pub fn follower_count_to_string_exact(count: &i32) -> String {
    count
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

pub fn view_count_to_string(count: &i64) -> String {
    debug!("view count: {}", count);
    if *count < 100 {
        count.to_string()
    } else if count / 1_000_000 > 0 {
        format!("{}M", truncate(*count as f64 / 1_000_000.0))
    } else if count / 1_000 > 0 {
        format!("{}K", truncate(*count as f64 / 1_000.0))
    } else {
        format!("{}", truncate(*count as f64))
    }
}

pub fn view_count_to_string_exact(count: &i64) -> String {
    count
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

#[derive(Clone, Debug)]
pub enum CookieValue {
    Bool(bool),
    Int(i64),
    String(String),
}

lazy_static! {
    pub static ref PREFERENCES_DEFAULT: HashMap<&'static str, CookieValue> = HashMap::from([
        ("thumbnails_for_feed", CookieValue::Bool(false)),
        ("thumbnails_for_all_videos", CookieValue::Bool(false)),
        ("expand_descriptions", CookieValue::Bool(false)),
        ("autoplay_videos", CookieValue::Bool(true)),
        ("exact_view_count", CookieValue::Bool(true)),
        ("exact_likes/dislikes_count", CookieValue::Bool(true)),
        ("likes/dislikes_on_channel_page", CookieValue::Bool(false)),
    ]);
}

impl fmt::Display for CookieValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookieValue::Bool(value) => write!(f, "{}", value),
            CookieValue::Int(value) => write!(f, "{}", value),
            CookieValue::String(value) => write!(f, "{}", value),
        }
    }
}

impl From<String> for CookieValue {
    fn from(value: String) -> Self {
        if let Ok(bool_value) = value.parse::<bool>() {
            CookieValue::Bool(bool_value)
        } else if let Ok(int_value) = value.parse::<i64>() {
            CookieValue::Int(int_value)
        } else {
            CookieValue::String(value)
        }
    }
}

pub fn get_cookies_from_request(req: &HttpRequest) -> HashMap<&str, CookieValue> {
    debug!("Received cookies: {:?}", req.cookies());
    PREFERENCES_DEFAULT
        .iter()
        .map(|(name, default_value)| {
            if let Some(cookie) = req.cookie(name) {
                (*name, CookieValue::from(cookie.value().to_string()))
            } else {
                (*name, default_value.clone())
            }
        })
        .collect()
}

pub fn preferences_to_cookies<'a>(
    mut preferences: HashMap<String, String>,
) -> HashMap<&'a str, CookieValue> {
    debug!("Received preferences: {:?}", preferences);
    PREFERENCES_DEFAULT
        .iter()
        .map(|(name, default_value)| {
            if let Some(value) = preferences.remove(*name) {
                (*name, CookieValue::from(value))
            } else if let Some(CookieValue::Bool(_)) = PREFERENCES_DEFAULT.get(*name) {
                (*name, CookieValue::Bool(false))
            } else {
                (*name, default_value.clone())
            }
        })
        .collect()
}

pub fn get_cookie_value_bool(req: &HttpRequest, cookie_name: &str) -> Result<bool, YtarsError> {
    let cookies_values = get_cookies_from_request(req);
    if let Some(CookieValue::Bool(value)) = cookies_values.get(cookie_name) {
        Ok(*value)
    } else if let Some(CookieValue::Bool(default_value)) = PREFERENCES_DEFAULT.get(cookie_name) {
        Ok(*default_value)
    } else {
        Err(YtarsError::Other(
            "Unexpected default value for show_thumbnails".to_string(),
        ))
    }
}
