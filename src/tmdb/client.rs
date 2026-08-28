use super::types::{
    MovieDetails, MovieSearchResult, SearchResponse, SeasonDetails, TmdbError, TvDetails,
    TvSearchResult,
};
use anyhow::{Context, Result, anyhow, bail};
use once_cell::sync::OnceCell;
use reqwest::{Client, Url};
use serde::de::DeserializeOwned;

static ACCESS_TOKEN: OnceCell<String> = OnceCell::new();
const BASE_URL: &str = "https://api.themoviedb.org/3/";

pub fn set_access_token(token: String) {
    ACCESS_TOKEN.set(token).expect("TMDB token set twice");
}

pub fn is_configured() -> bool {
    ACCESS_TOKEN.get().is_some()
}

pub async fn search_movies(
    query: &str,
    year: Option<u32>,
    language: &str,
) -> Result<SearchResponse<MovieSearchResult>> {
    let mut params = vec![
        ("query", query.to_string()),
        ("language", language.to_string()),
        ("include_adult", "true".to_string()),
    ];
    if let Some(year) = year {
        params.push(("year", year.to_string()));
    }
    get("search/movie", &params).await
}

pub async fn search_tv(
    query: &str,
    year: Option<u32>,
    language: &str,
) -> Result<SearchResponse<TvSearchResult>> {
    let mut params = vec![
        ("query", query.to_string()),
        ("language", language.to_string()),
        ("include_adult", "true".to_string()),
    ];
    if let Some(year) = year {
        params.push(("first_air_date_year", year.to_string()));
    }
    get("search/tv", &params).await
}

pub async fn get_movie(id: u32, language: &str) -> Result<MovieDetails> {
    get(
        &format!("movie/{id}"),
        &[
            ("language", language.to_string()),
            (
                "append_to_response",
                "credits,external_ids,keywords".to_string(),
            ),
        ],
    )
    .await
}

pub async fn get_tv(id: u32, language: &str) -> Result<TvDetails> {
    get(
        &format!("tv/{id}"),
        &[
            ("language", language.to_string()),
            (
                "append_to_response",
                "credits,external_ids,keywords".to_string(),
            ),
        ],
    )
    .await
}

pub async fn get_tv_season(id: u32, season: u32, language: &str) -> Result<SeasonDetails> {
    get(
        &format!("tv/{id}/season/{season}"),
        &[("language", language.to_string())],
    )
    .await
}

async fn get<T: DeserializeOwned>(path: &str, params: &[(&str, String)]) -> Result<T> {
    let token = ACCESS_TOKEN
        .get()
        .ok_or_else(|| anyhow!("TMDB access token is not configured; use --tmdb-token"))?;
    let url = Url::parse(BASE_URL)?.join(path)?;
    let client = Client::builder()
        .user_agent(format!(
            "Dantalian/{} (https://github.com/nanozuki/dantalian)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let response = client
        .get(url)
        .bearer_auth(token)
        .query(params)
        .send()
        .await
        .with_context(|| format!("request TMDB {path}"))?;
    let status = response.status();
    let bytes = response.bytes().await.with_context(|| "read TMDB body")?;
    if !status.is_success() {
        if let Ok(error) = serde_json::from_slice::<TmdbError>(&bytes) {
            bail!(error);
        }
        bail!(
            "TMDB request returned {}: {}",
            status,
            String::from_utf8_lossy(&bytes)
        );
    }
    serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "deserialize TMDB response for {path}: {}",
            String::from_utf8_lossy(&bytes)
        )
    })
}
