mod client;
mod types;

pub use client::{
    get_movie, get_tv, get_tv_season, is_configured, search_movies, search_tv, set_access_token,
};
pub use types::*;

pub const DEFAULT_LANGUAGE: &str = "zh-CN";

pub fn image_url(path: &str) -> String {
    format!("https://image.tmdb.org/t/p/original{path}")
}
