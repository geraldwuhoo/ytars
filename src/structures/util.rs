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

fn truncate(num: f64) -> f64 {
    let num_integ = num.trunc().to_string();
    let num_fract = num.fract().to_string();
    let significant_digits = 3;

    if num_integ.len() == significant_digits {
        return num.trunc()
    }
    else {
        let decimals = significant_digits - num_integ.len() + 2 as usize;
        let combined_number: f64 = (num_integ + "." + &num_fract[2..decimals]).parse().unwrap();
        return combined_number
    }
}

pub fn follower_count_to_string(count: &i32) -> String {
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

pub fn view_count_to_string(count: &i64) -> String {
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
