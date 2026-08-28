use serde::Deserialize;
use std::fmt;

const TMDB_WEB: &str = "https://www.themoviedb.org";

#[derive(Deserialize, Debug)]
pub struct TmdbError {
    pub status_message: String,
    pub status_code: Option<u32>,
}

impl fmt::Display for TmdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status_code {
            Some(code) => write!(f, "TMDB error {}: {}", code, self.status_message),
            None => write!(f, "TMDB error: {}", self.status_message),
        }
    }
}

impl std::error::Error for TmdbError {}

#[derive(Deserialize, Debug)]
pub struct SearchResponse<T> {
    pub page: u32,
    pub total_pages: u32,
    pub total_results: u32,
    pub results: Vec<T>,
}

#[derive(Deserialize, Debug)]
pub struct MovieSearchResult {
    pub id: u32,
    pub title: String,
    pub original_title: String,
    pub release_date: Option<String>,
    pub original_language: String,
    pub overview: String,
}

impl MovieSearchResult {
    pub fn url(&self) -> String {
        format!("{TMDB_WEB}/movie/{}", self.id)
    }
}

impl fmt::Display for MovieSearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "* {} / {}\n  TMDB ID: {}\n  Release Date: {}\n  Original Language: {}\n  URL: {}",
            self.title,
            self.original_title,
            self.id,
            self.release_date.as_deref().unwrap_or("*"),
            self.original_language,
            self.url()
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct TvSearchResult {
    pub id: u32,
    pub name: String,
    pub original_name: String,
    pub first_air_date: Option<String>,
    pub original_language: String,
    pub overview: String,
}

impl TvSearchResult {
    pub fn url(&self) -> String {
        format!("{TMDB_WEB}/tv/{}", self.id)
    }
}

impl fmt::Display for TvSearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "* {} / {}\n  TMDB ID: {}\n  First Air Date: {}\n  Original Language: {}\n  URL: {}",
            self.name,
            self.original_name,
            self.id,
            self.first_air_date.as_deref().unwrap_or("*"),
            self.original_language,
            self.url()
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Genre {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Company {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Person {
    pub name: String,
    pub character: Option<String>,
    pub job: Option<String>,
    pub order: Option<u32>,
    pub profile_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Credits {
    #[serde(default)]
    pub cast: Vec<Person>,
    #[serde(default)]
    pub crew: Vec<Person>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Keyword {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MovieKeywords {
    #[serde(default)]
    pub keywords: Vec<Keyword>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct TvKeywords {
    #[serde(default)]
    pub results: Vec<Keyword>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MovieDetails {
    pub id: u32,
    pub title: String,
    pub original_title: String,
    pub original_language: String,
    pub overview: String,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    pub vote_average: f64,
    pub vote_count: u32,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub production_companies: Vec<Company>,
    #[serde(default)]
    pub credits: Credits,
    #[serde(default)]
    pub external_ids: ExternalIds,
    #[serde(default)]
    pub keywords: MovieKeywords,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SeasonSummary {
    pub season_number: u32,
    pub name: String,
    pub episode_count: u32,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TvDetails {
    pub id: u32,
    pub name: String,
    pub original_name: String,
    pub original_language: String,
    pub overview: String,
    pub first_air_date: Option<String>,
    #[serde(default)]
    pub episode_run_time: Vec<u32>,
    pub number_of_episodes: u32,
    pub number_of_seasons: u32,
    #[serde(default)]
    pub genres: Vec<Genre>,
    pub vote_average: f64,
    pub vote_count: u32,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub production_companies: Vec<Company>,
    #[serde(default)]
    pub credits: Credits,
    #[serde(default)]
    pub external_ids: ExternalIds,
    #[serde(default)]
    pub keywords: TvKeywords,
    #[serde(default)]
    pub seasons: Vec<SeasonSummary>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EpisodeDetails {
    pub id: u32,
    pub name: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub episode_number: u32,
    pub season_number: u32,
    pub runtime: Option<u32>,
    pub vote_average: f64,
    pub vote_count: u32,
    pub still_path: Option<String>,
    #[serde(default)]
    pub crew: Vec<Person>,
    #[serde(default)]
    pub guest_stars: Vec<Person>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SeasonDetails {
    pub id: u32,
    pub name: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    pub season_number: u32,
    #[serde(default)]
    pub episodes: Vec<EpisodeDetails>,
}
