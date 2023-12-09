use super::model::VideoType;

pub const fn _default_false() -> bool {
    false
}

pub const fn _default_count() -> i64 {
    100
}

pub const fn _default_video_type() -> VideoType {
    VideoType::Video
}

pub fn follower_count_to_string(count: &i32) -> String {
    if count / 1_000_000 > 0 {
        format!("{}M", *count as f32 / 1_000_000.0)
    } else if count / 1_000 > 0 {
        format!("{}K", *count as f32 / 1_000.0)
    } else {
        format!("{}", count)
    }
}
