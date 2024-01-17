use std::collections::HashMap;

use actix_web::{cookie::Cookie, get, post, web, HttpRequest, HttpResponse, Result};
use askama::Template;

use crate::structures::{
    errors::YtarsError,
    util::{get_cookies_from_request, preferences_to_cookies, CookieValue},
};

mod filters {
    pub fn pretty_print_cookie<T: std::fmt::Display>(s: T) -> ::askama::Result<String> {
        let s = s.to_string();
        Ok(s.replace('_', " "))
    }
}

#[derive(Debug, Template)]
#[template(path = "preferences.html")]
struct PreferencesTemplate<'a> {
    cookies: Vec<(Cookie<'a>, CookieValue)>,
}

fn build_response(cookies_values: HashMap<&str, CookieValue>) -> Result<HttpResponse, YtarsError> {
    let mut cookies: Vec<(Cookie, CookieValue)> = cookies_values
        .into_iter()
        .map(|(name, value)| (Cookie::new(name, value.to_string()), value))
        .collect();
    cookies.sort_by(|(cookie_a, _), (cookie_b, _)| cookie_a.name().cmp(cookie_b.name()));

    // Build response and append all cookies
    let response = &mut HttpResponse::Ok();

    for (cookie, _) in &cookies {
        response.cookie(cookie.clone());
    }

    Ok(response
        .content_type("text/html")
        .body(PreferencesTemplate { cookies }.render()?))
}

#[get("/preferences")]
pub async fn preferences_get_handler(req: HttpRequest) -> Result<HttpResponse, YtarsError> {
    let cookies_values = get_cookies_from_request(&req);
    build_response(cookies_values)
}

#[post("/preferences")]
pub async fn preferences_post_handler(
    web::Form(preferences): web::Form<HashMap<String, String>>,
) -> Result<HttpResponse, YtarsError> {
    let cookies_values = preferences_to_cookies(preferences);
    build_response(cookies_values)
}
