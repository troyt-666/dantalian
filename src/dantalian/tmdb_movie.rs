use super::tmdb_common::{TmdbConfig, download_file};
use super::utils::is_video_file;
use crate::nfogen::Generator;
use crate::nfogen::nfo::{Actor, MOVIE_NFO_NAME, Movie, Rating, UniqueId};
use crate::tmdb::{MovieDetails, get_movie, image_url};
use crate::{error, info, warn};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const POSTER_NAME: &str = "poster.jpg";
const FANART_NAME: &str = "fanart.jpg";

pub async fn dantalian_tmdb_movie<F: Fn(String) -> bool>(
    source: &Path,
    is_force: &F,
) -> Result<()> {
    info!("Run TMDB movie scraper for {}", source.to_string_lossy());
    let generator = Generator::new();
    for entry in WalkDir::new(source).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            let path = entry.path().to_string_lossy().to_string();
            info!(ind: 1, "Check {} ...", path);
            match handle_dir(entry.path(), is_force(path), &generator).await {
                Ok(_) => info!(ind: 2, "Completed!"),
                Err(error) => error!(
                    ind: 2,
                    "Failed: {}\n{}",
                    error,
                    error.root_cause()
                ),
            }
        }
    }
    Ok(())
}

async fn handle_dir(path: &Path, force: bool, generator: &Generator<'_>) -> Result<()> {
    let config = TmdbConfig::parse_movie(path).await?;
    let nfo_paths = movie_nfo_paths(path)?;
    let poster_path = path.join(POSTER_NAME);
    let fanart_path = path.join(FANART_NAME);
    if !force
        && !nfo_paths.is_empty()
        && nfo_paths.iter().all(|path| path.exists())
        && poster_path.exists()
        && fanart_path.exists()
    {
        return Ok(());
    }

    let details = localized_movie(&config).await?;
    info!(
        ind: 2,
        "Matched TMDB movie [{}] {} / {}",
        details.id,
        details.title,
        details.original_title
    );
    let movie = movie_from_tmdb(details);
    let movie_nfo = generator.gen_movie_nfo(&movie)?;
    for nfo_path in nfo_paths {
        if force || !nfo_path.exists() {
            let mut file = File::create(&nfo_path)?;
            file.write_all(movie_nfo.as_bytes())?;
            info!(ind: 2, "Generated {}", nfo_path.to_string_lossy());
        }
    }
    download_artwork(movie.poster.as_deref(), &poster_path, force).await;
    download_artwork(movie.fanart.as_deref(), &fanart_path, force).await;
    Ok(())
}

async fn localized_movie(config: &TmdbConfig) -> Result<MovieDetails> {
    let mut movie = get_movie(config.tmdb_id, &config.language)
        .await
        .with_context(|| format!("get TMDB movie {}", config.tmdb_id))?;
    for language in config.languages(&movie.original_language) {
        if !movie.title.trim().is_empty() && !movie.overview.trim().is_empty() {
            break;
        }
        let fallback = get_movie(config.tmdb_id, &language).await?;
        fill_if_empty(&mut movie.title, fallback.title);
        fill_if_empty(&mut movie.overview, fallback.overview);
    }
    Ok(movie)
}

fn movie_from_tmdb(details: MovieDetails) -> Movie {
    let mut unique_ids = vec![UniqueId {
        kind: "tmdb".into(),
        is_default: true,
        value: details.id.to_string(),
    }];
    if let Some(imdb_id) = details
        .external_ids
        .imdb_id
        .clone()
        .filter(|id| !id.is_empty())
    {
        unique_ids.push(UniqueId {
            kind: "imdb".into(),
            is_default: false,
            value: imdb_id,
        });
    }
    let actors = details
        .credits
        .cast
        .iter()
        .take(40)
        .map(|person| Actor {
            name: person.name.clone(),
            role: person.character.clone().unwrap_or_default(),
            order: person.order.unwrap_or(0),
            thumb: person
                .profile_path
                .as_deref()
                .map(image_url)
                .unwrap_or_default(),
        })
        .collect();
    let directors = crew_names(&details, &["Director"]);
    let credits = crew_names(
        &details,
        &["Writer", "Screenplay", "Story", "Novel", "Characters"],
    );
    let premiered = details.release_date.unwrap_or_default();
    let year = premiered.get(..4).and_then(|value| value.parse().ok());
    Movie {
        title: nonempty_or(details.title, &details.original_title),
        original_title: details.original_title,
        ratings: vec![Rating {
            name: "tmdb".into(),
            max: 10,
            is_default: true,
            value: details.vote_average,
            votes: Some(details.vote_count),
        }],
        unique_ids,
        plot: details.overview,
        poster: details.poster_path.as_deref().map(image_url),
        fanart: details.backdrop_path.as_deref().map(image_url),
        year,
        runtime: details.runtime,
        genres: details.genres.into_iter().map(|genre| genre.name).collect(),
        tags: details
            .keywords
            .keywords
            .into_iter()
            .map(|keyword| keyword.name)
            .collect(),
        premiered,
        status: details.status,
        studio: details
            .production_companies
            .first()
            .map(|company| company.name.clone()),
        directors,
        credits,
        actors,
    }
}

fn crew_names(details: &MovieDetails, jobs: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    for person in &details.credits.crew {
        if person.job.as_deref().is_some_and(|job| jobs.contains(&job))
            && !names.contains(&person.name)
        {
            names.push(person.name.clone());
        }
    }
    names
}

fn movie_nfo_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = HashSet::from([path.join(MOVIE_NFO_NAME)]);
    for entry in WalkDir::new(path).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.file_type().is_file() && is_video_file(entry.path()) {
            paths.insert(entry.path().with_extension("nfo"));
        }
    }
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort();
    Ok(paths)
}

async fn download_artwork(url: Option<&str>, path: &Path, force: bool) {
    if let Some(url) = url.filter(|_| force || !path.exists())
        && let Err(error) = download_file(url, path).await
    {
        warn!(
            ind: 2,
            "Could not download artwork to {}: {}",
            path.to_string_lossy(),
            error
        );
    }
}

fn fill_if_empty(destination: &mut String, fallback: String) {
    if destination.trim().is_empty() {
        *destination = fallback;
    }
}

fn nonempty_or(value: String, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmdb::{Credits, ExternalIds, Genre, MovieKeywords};

    #[test]
    fn maps_tmdb_movie_to_provider_neutral_nfo() {
        let movie = movie_from_tmdb(MovieDetails {
            id: 123,
            title: "中文标题".into(),
            original_title: "Original".into(),
            original_language: "en".into(),
            overview: "简介".into(),
            release_date: Some("2024-01-02".into()),
            runtime: Some(101),
            genres: vec![Genre {
                name: "剧情".into(),
            }],
            vote_average: 8.2,
            vote_count: 42,
            poster_path: Some("/poster.jpg".into()),
            backdrop_path: Some("/fanart.jpg".into()),
            status: Some("Released".into()),
            production_companies: vec![],
            credits: Credits::default(),
            external_ids: ExternalIds {
                imdb_id: Some("tt123".into()),
                tvdb_id: None,
            },
            keywords: MovieKeywords::default(),
        });
        assert_eq!(movie.title, "中文标题");
        assert_eq!(movie.year, Some(2024));
        assert_eq!(movie.unique_ids.len(), 2);
        assert_eq!(
            movie.poster.as_deref(),
            Some("https://image.tmdb.org/t/p/original/poster.jpg")
        );
    }
}
