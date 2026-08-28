use super::tmdb_common::{TmdbConfig, download_file, locale_for_original_language};
use super::utils::is_video_file;
use crate::nfogen::{Actor, Episode, Generator, Rating, TVSHOW_NFO_NAME, TVShow, UniqueId};
use crate::tmdb::{EpisodeDetails, SeasonDetails, TvDetails, get_tv, get_tv_season, image_url};
use crate::{error, info, warn};
use anyhow::{Context, Result, anyhow, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const POSTER_NAME: &str = "poster.jpg";
const FANART_NAME: &str = "fanart.jpg";
static EPISODE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)s(?P<season>\d{1,2})e(?P<episode>\d{1,3})").unwrap());
static EXTRA_EPISODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)e\d{1,3}").unwrap());

struct EpisodeJob {
    season: u32,
    episode: u32,
    video_path: PathBuf,
    nfo_path: PathBuf,
    thumb_path: PathBuf,
}

pub async fn dantalian_tmdb_tv<F: Fn(String) -> bool>(source: &Path, is_force: &F) -> Result<()> {
    info!("Run TMDB TV scraper for {}", source.to_string_lossy());
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
    let config = TmdbConfig::parse_tv(path).await?;
    let jobs = episode_jobs(path, force, &config)?;
    let tvshow_path = path.join(TVSHOW_NFO_NAME);
    let poster_path = path.join(POSTER_NAME);
    let fanart_path = path.join(FANART_NAME);
    if jobs.is_empty()
        && !force
        && tvshow_path.exists()
        && poster_path.exists()
        && fanart_path.exists()
    {
        return Ok(());
    }

    let details = localized_tv(&config).await?;
    info!(
        ind: 2,
        "Matched TMDB TV [{}] {} / {}",
        details.id,
        details.name,
        details.original_name
    );
    let show = tvshow_from_tmdb(&details);
    if force || !tvshow_path.exists() {
        let mut file = File::create(&tvshow_path)?;
        file.write_all(generator.gen_tvshow_nfo(&show)?.as_bytes())?;
        info!(ind: 2, "Generated {}", tvshow_path.to_string_lossy());
    }
    download_artwork(show.poster.as_deref(), &poster_path, force).await;
    download_artwork(show.fanart.as_deref(), &fanart_path, force).await;

    let jobs_by_season = group_jobs(jobs);
    for (season_number, jobs) in jobs_by_season {
        let localized = localized_season(&config, &details, season_number).await?;
        download_season_poster(path, &localized.primary, force).await;
        let primary_by_episode: HashMap<u32, &EpisodeDetails> = localized
            .primary
            .episodes
            .iter()
            .map(|episode| (episode.episode_number, episode))
            .collect();
        let original_by_episode: HashMap<u32, &EpisodeDetails> = localized
            .original
            .episodes
            .iter()
            .map(|episode| (episode.episode_number, episode))
            .collect();
        for job in jobs {
            let primary = primary_by_episode.get(&job.episode).ok_or_else(|| {
                anyhow!(
                    "TMDB TV {} season {} has no episode {} for {}",
                    config.tmdb_id,
                    job.season,
                    job.episode,
                    job.video_path.to_string_lossy()
                )
            })?;
            let original = original_by_episode
                .get(&job.episode)
                .copied()
                .unwrap_or(primary);
            let episode = episode_from_tmdb(&details, primary, original);
            let mut file = File::create(&job.nfo_path)?;
            file.write_all(generator.gen_episode_nfo(&episode)?.as_bytes())?;
            info!(ind: 2, "Generated {}", job.nfo_path.to_string_lossy());
            download_artwork(episode.thumb.as_deref(), &job.thumb_path, force).await;
        }
    }
    Ok(())
}

struct LocalizedSeason {
    primary: SeasonDetails,
    original: SeasonDetails,
}

async fn localized_tv(config: &TmdbConfig) -> Result<TvDetails> {
    let mut show = get_tv(config.tmdb_id, &config.language)
        .await
        .with_context(|| format!("get TMDB TV {}", config.tmdb_id))?;
    for language in config.languages(&show.original_language) {
        if !show.name.trim().is_empty() && !show.overview.trim().is_empty() {
            break;
        }
        let fallback = get_tv(config.tmdb_id, &language).await?;
        fill_if_empty(&mut show.name, fallback.name);
        fill_if_empty(&mut show.overview, fallback.overview);
    }
    Ok(show)
}

async fn localized_season(
    config: &TmdbConfig,
    show: &TvDetails,
    season: u32,
) -> Result<LocalizedSeason> {
    let mut primary = get_tv_season(config.tmdb_id, season, &config.language).await?;
    let original_language = locale_for_original_language(&show.original_language);
    let original = if original_language == config.language {
        primary.clone()
    } else {
        get_tv_season(config.tmdb_id, season, &original_language).await?
    };
    merge_season_missing(&mut primary, &original);
    for language in config.languages(&show.original_language) {
        if season_is_complete(&primary) {
            break;
        }
        if language == original_language {
            continue;
        }
        let fallback = get_tv_season(config.tmdb_id, season, &language).await?;
        merge_season_missing(&mut primary, &fallback);
    }
    Ok(LocalizedSeason { primary, original })
}

fn merge_season_missing(destination: &mut SeasonDetails, fallback: &SeasonDetails) {
    fill_if_empty(&mut destination.name, fallback.name.clone());
    fill_if_empty(&mut destination.overview, fallback.overview.clone());
    let fallback_episodes: HashMap<u32, &EpisodeDetails> = fallback
        .episodes
        .iter()
        .map(|episode| (episode.episode_number, episode))
        .collect();
    for episode in &mut destination.episodes {
        if let Some(fallback) = fallback_episodes.get(&episode.episode_number) {
            fill_if_empty(&mut episode.name, fallback.name.clone());
            fill_if_empty(&mut episode.overview, fallback.overview.clone());
        }
    }
}

fn season_is_complete(season: &SeasonDetails) -> bool {
    !season.name.trim().is_empty()
        && season
            .episodes
            .iter()
            .all(|episode| !episode.name.trim().is_empty() && !episode.overview.trim().is_empty())
}

fn tvshow_from_tmdb(details: &TvDetails) -> TVShow {
    let mut unique_ids = vec![UniqueId {
        kind: "tmdb".into(),
        is_default: true,
        value: details.id.to_string(),
    }];
    if let Some(imdb_id) = details
        .external_ids
        .imdb_id
        .as_ref()
        .filter(|id| !id.is_empty())
    {
        unique_ids.push(UniqueId {
            kind: "imdb".into(),
            is_default: false,
            value: imdb_id.clone(),
        });
    }
    if let Some(tvdb_id) = details.external_ids.tvdb_id {
        unique_ids.push(UniqueId {
            kind: "tvdb".into(),
            is_default: false,
            value: tvdb_id.to_string(),
        });
    }
    TVShow {
        title: nonempty_or(&details.name, &details.original_name),
        original_title: details.original_name.clone(),
        ratings: vec![Rating {
            name: "tmdb".into(),
            max: 10,
            is_default: true,
            value: details.vote_average,
            votes: Some(details.vote_count),
        }],
        unique_ids,
        season_count: details.number_of_seasons,
        eps_count: Some(details.number_of_episodes),
        plot: details.overview.clone(),
        poster: details.poster_path.as_deref().map(image_url),
        fanart: details.backdrop_path.as_deref().map(image_url),
        genres: details
            .genres
            .iter()
            .map(|genre| genre.name.clone())
            .collect(),
        tags: details
            .keywords
            .results
            .iter()
            .map(|keyword| keyword.name.clone())
            .collect(),
        premiered: details.first_air_date.clone().unwrap_or_default(),
        status: details.status.clone(),
        studio: details
            .production_companies
            .first()
            .map(|company| company.name.clone()),
        actors: details
            .credits
            .cast
            .iter()
            .take(40)
            .map(actor_from_tmdb)
            .collect::<Vec<_>>()
            .into(),
    }
}

fn episode_from_tmdb(
    show: &TvDetails,
    primary: &EpisodeDetails,
    original: &EpisodeDetails,
) -> Episode {
    let directors = person_names(&primary.crew, &["Director"]);
    let credits = person_names(
        &primary.crew,
        &["Writer", "Screenplay", "Story", "Teleplay"],
    );
    let aired = primary.air_date.clone().unwrap_or_default();
    Episode {
        title: nonempty_or(&primary.name, &original.name),
        original_title: nonempty_or(&original.name, &primary.name),
        show_title: nonempty_or(&show.name, &show.original_name),
        ratings: if primary.vote_count > 0 {
            vec![Rating {
                name: "tmdb".into(),
                max: 10,
                is_default: true,
                value: primary.vote_average,
                votes: Some(primary.vote_count),
            }]
        } else {
            vec![]
        },
        unique_ids: vec![UniqueId {
            kind: "tmdb".into(),
            is_default: true,
            value: primary.id.to_string(),
        }],
        season: primary.season_number,
        ep_index: primary.episode_number.to_string(),
        is_sp: primary.season_number == 0,
        plot: primary.overview.clone(),
        thumb: primary.still_path.as_deref().map(image_url),
        runtime: primary.runtime,
        directors: directors.into(),
        credits: credits.into(),
        premiered: aired.clone(),
        status: None,
        aired: Some(aired),
        studio: show
            .production_companies
            .first()
            .map(|company| company.name.clone()),
        actors: primary
            .guest_stars
            .iter()
            .map(actor_from_tmdb)
            .collect::<Vec<_>>()
            .into(),
    }
}

fn actor_from_tmdb(person: &crate::tmdb::Person) -> Actor {
    Actor {
        name: person.name.clone(),
        role: person.character.clone().unwrap_or_default(),
        order: person.order.unwrap_or(0),
        thumb: person
            .profile_path
            .as_deref()
            .map(image_url)
            .unwrap_or_default(),
    }
}

fn person_names(persons: &[crate::tmdb::Person], jobs: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    for person in persons {
        if person.job.as_deref().is_some_and(|job| jobs.contains(&job))
            && !names.contains(&person.name)
        {
            names.push(person.name.clone());
        }
    }
    names
}

fn episode_jobs(path: &Path, force: bool, config: &TmdbConfig) -> Result<Vec<EpisodeJob>> {
    let episode_re = config.episode_re.as_ref().unwrap_or(&EPISODE_RE);
    let mut jobs = Vec::new();
    for entry in WalkDir::new(path).min_depth(1).max_depth(4) {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_video_file(entry.path()) {
            continue;
        }
        let Some(filename) = entry.file_name().to_str() else {
            continue;
        };
        let Some(captures) = episode_re.captures(filename) else {
            warn!(
                ind: 2,
                "Skip video without SxxEyy number: {}",
                entry.path().to_string_lossy()
            );
            continue;
        };
        let whole_match = captures.get(0).expect("episode regex has full match");
        if EXTRA_EPISODE_RE.is_match(&filename[whole_match.end()..]) {
            bail!(
                "multi-episode filename is not supported yet: {}",
                entry.path().to_string_lossy()
            );
        }
        let season = captures
            .name("season")
            .map(|value| value.as_str().parse())
            .transpose()?
            .or(config.default_season)
            .ok_or_else(|| {
                anyhow!(
                    "episode_re matched without a season; set default_season in {}",
                    path.join("dantalian.toml").to_string_lossy()
                )
            })?;
        let episode = captures
            .name("episode")
            .ok_or_else(|| anyhow!("episode_re must define an episode capture group"))?
            .as_str()
            .parse()?;
        let nfo_path = entry.path().with_extension("nfo");
        if !force && nfo_path.exists() {
            continue;
        }
        let stem = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| anyhow!("invalid episode filename"))?;
        jobs.push(EpisodeJob {
            season,
            episode,
            video_path: entry.path().to_path_buf(),
            nfo_path,
            thumb_path: entry.path().with_file_name(format!("{stem}-thumb.jpg")),
        });
    }
    Ok(jobs)
}

fn group_jobs(jobs: Vec<EpisodeJob>) -> BTreeMap<u32, Vec<EpisodeJob>> {
    let mut grouped: BTreeMap<u32, Vec<EpisodeJob>> = BTreeMap::new();
    for job in jobs {
        grouped.entry(job.season).or_default().push(job);
    }
    grouped
}

async fn download_season_poster(path: &Path, season: &SeasonDetails, force: bool) {
    let destination = path.join(format!("season{:02}-poster.jpg", season.season_number));
    let url = season.poster_path.as_deref().map(image_url);
    download_artwork(url.as_deref(), &destination, force).await;
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

fn nonempty_or(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn finds_nested_sxxeyy_episode_files() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dantalian-tv-{nonce}"));
        let season = root.join("Season 01");
        std::fs::create_dir_all(&season).unwrap();
        File::create(season.join("Example S01E02.mkv")).unwrap();
        File::create(season.join("readme.txt")).unwrap();
        let config = TmdbConfig {
            tmdb_id: 1,
            language: "zh-CN".into(),
            fallback_languages: vec![],
            episode_re: None,
            default_season: None,
        };
        let jobs = episode_jobs(&root, false, &config).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].season, 1);
        assert_eq!(jobs[0].episode, 2);
        assert!(jobs[0].nfo_path.ends_with("Example S01E02.nfo"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
