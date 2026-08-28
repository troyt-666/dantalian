use crate::info;
use crate::tmdb::{DEFAULT_LANGUAGE, search_movies, search_tv};
use anyhow::{Context, Result, anyhow, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub const DIR_CONFIG_NAME: &str = "dantalian.toml";

#[derive(Deserialize, Serialize, Debug, Default)]
struct TmdbConfigFile {
    tmdb_id: Option<u32>,
    language: Option<String>,
    fallback_languages: Option<Vec<String>>,
    episode_re: Option<String>,
    default_season: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TmdbConfig {
    pub tmdb_id: u32,
    pub language: String,
    pub fallback_languages: Vec<String>,
    pub episode_re: Option<Regex>,
    pub default_season: Option<u32>,
}

impl TmdbConfig {
    pub async fn parse_movie(path: &Path) -> Result<Self> {
        Self::parse(path, MediaKind::Movie).await
    }

    pub async fn parse_tv(path: &Path) -> Result<Self> {
        Self::parse(path, MediaKind::Tv).await
    }

    async fn parse(path: &Path, kind: MediaKind) -> Result<Self> {
        let filepath = path.join(DIR_CONFIG_NAME);
        if filepath.exists() {
            let body = std::fs::read_to_string(&filepath)?;
            let config: TmdbConfigFile = toml::from_str(&body)
                .with_context(|| format!("parse {}", filepath.to_string_lossy()))?;
            let tmdb_id = config.tmdb_id.ok_or_else(|| {
                anyhow!(
                    "{} does not contain tmdb_id; refusing to replace an existing config",
                    filepath.to_string_lossy()
                )
            })?;
            return Self::from_file(tmdb_id, config);
        }

        let dirname = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("invalid media folder name"))?;
        let (query, year) = media_query(dirname);
        info!(ind: 2, "Search TMDB for {:?}, year {:?}", query, year);
        let tmdb_id = match kind {
            MediaKind::Movie => search_movies(&query, year, DEFAULT_LANGUAGE)
                .await?
                .results
                .into_iter()
                .next()
                .map(|item| item.id),
            MediaKind::Tv => search_tv(&query, year, DEFAULT_LANGUAGE)
                .await?
                .results
                .into_iter()
                .next()
                .map(|item| item.id),
        }
        .ok_or_else(|| anyhow!("TMDB did not find {:?}", query))?;
        let config = TmdbConfigFile {
            tmdb_id: Some(tmdb_id),
            language: Some(DEFAULT_LANGUAGE.into()),
            fallback_languages: None,
            episode_re: None,
            default_season: None,
        };
        let mut file = File::create(&filepath)?;
        file.write_all(toml::to_string(&config)?.as_bytes())?;
        info!(
            ind: 2,
            "Created {} with TMDB ID {}",
            filepath.to_string_lossy(),
            tmdb_id
        );
        Self::from_file(tmdb_id, config)
    }

    fn from_file(tmdb_id: u32, config: TmdbConfigFile) -> Result<Self> {
        let episode_re = config
            .episode_re
            .map(|pattern| Regex::new(&pattern))
            .transpose()?;
        Ok(Self {
            tmdb_id,
            language: config.language.unwrap_or_else(|| DEFAULT_LANGUAGE.into()),
            fallback_languages: config.fallback_languages.unwrap_or_default(),
            episode_re,
            default_season: config.default_season,
        })
    }

    pub fn languages(&self, original_language: &str) -> Vec<String> {
        let mut languages = self.fallback_languages.clone();
        if languages.is_empty() {
            languages.push(locale_for_original_language(original_language));
            languages.push("en-US".into());
        }
        languages.retain(|language| language != &self.language);
        languages.dedup();
        languages
    }
}

#[derive(Clone, Copy)]
enum MediaKind {
    Movie,
    Tv,
}

static YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:^|[\s._\[(])(19\d{2}|20\d{2})(?:$|[\s._\])])").unwrap());
static BRACKETED_YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\[(](19\d{2}|20\d{2})[\])]").unwrap());
static TECH_TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)[\[(](?:2160p|1080p|720p|480p|uhd|bluray|blu-ray|bdrip|webrip|web-dl|remux|x26[45]|h\.26[45]|hevc|avc|hdr|dv|dolby|aac|dts[^\])]*)[\])]",
    )
    .unwrap()
});

pub fn media_query(dirname: &str) -> (String, Option<u32>) {
    let year_match = BRACKETED_YEAR_RE
        .captures_iter(dirname)
        .last()
        .or_else(|| YEAR_RE.captures_iter(dirname).last());
    let year = year_match
        .as_ref()
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse().ok());
    let query_part = year_match
        .as_ref()
        .and_then(|captures| captures.get(0))
        .map(|matched| {
            if matched.start() > 0 {
                &dirname[..matched.start()]
            } else {
                &dirname[matched.end()..]
            }
        })
        .unwrap_or(dirname);
    let without_tech = TECH_TAG_RE.replace_all(query_part, " ");
    let query = without_tech
        .replace(['.', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|character| matches!(character, '-' | '[' | ']' | '(' | ')'))
        .trim()
        .to_string();
    (query, year)
}

pub fn locale_for_original_language(language: &str) -> String {
    match language {
        "ja" => "ja-JP",
        "zh" => "zh-CN",
        "ko" => "ko-KR",
        "en" => "en-US",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "es" => "es-ES",
        "it" => "it-IT",
        "pt" => "pt-BR",
        "ru" => "ru-RU",
        other => other,
    }
    .to_string()
}

pub async fn download_file(url: &str, destination: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("request artwork {url}"))?;
    if !response.status().is_success() {
        bail!("artwork request returned {}", response.status());
    }
    let bytes = response
        .bytes()
        .await
        .with_context(|| "read artwork body")?;
    std::fs::write(destination, bytes)?;
    info!(ind: 2, "Downloaded {}", destination.to_string_lossy());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_title_and_year_from_common_folder_names() {
        assert_eq!(
            media_query("Blade Runner 2049 (2017) [2160p][BluRay]"),
            ("Blade Runner 2049".into(), Some(2017))
        );
        assert_eq!(
            media_query("The.Expanse.2015.1080p"),
            ("The Expanse".into(), Some(2015))
        );
    }
}
