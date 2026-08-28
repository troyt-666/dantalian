use crate::bangumi::get_anime_data;
use crate::dantalian::Config;
use crate::dantalian::utils::is_video_file;
use crate::nfogen::Generator;
use crate::nfogen::nfo::{MOVIE_NFO_NAME, Movie};
use crate::{error, info, warn};
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MOVIE_POSTER_NAME: &str = "poster.jpg";

pub async fn dantalian_movie<F: Fn(String) -> bool>(source: &Path, is_force: &F) -> Result<()> {
    info!("Run dantalian for {}", source.to_string_lossy());
    let generator = Generator::new();
    for e in WalkDir::new(source).min_depth(1).max_depth(1) {
        let entry = e?;
        if entry.file_type().is_dir() {
            let path = entry.path().to_string_lossy().to_string();
            info!(ind: 1, "Check {} ...", path);
            match handle_dir(entry.path(), is_force(path), &generator).await {
                Ok(_) => info!(ind: 2, "Completed!"),
                Err(e) => error!(ind: 2, "Failed: {}\n{}", e, e.root_cause()),
            };
        }
    }
    Ok(())
}

async fn handle_dir<'a>(path: &Path, force: bool, generator: &'a Generator<'a>) -> Result<()> {
    let subject_id = Config::parse_movie_subject(path).await?;
    let nfo_paths = movie_nfo_paths(path)?;
    let poster_path = path.join(MOVIE_POSTER_NAME);
    if !force && nfo_paths.iter().all(|path| path.exists()) && poster_path.exists() {
        return Ok(());
    }

    let movie_data = get_anime_data(subject_id)
        .await
        .with_context(|| "get_movie_info")?;
    warn_if_non_theatrical_platform(&movie_data.subject.platform);
    let movie = Movie::from_bgm(movie_data);
    let poster_url = movie.poster.clone();
    let movie_nfo = generator.gen_movie_nfo(&movie)?;

    for nfo_path in nfo_paths {
        if force || !nfo_path.exists() {
            let mut nfo_file = File::create(&nfo_path)?;
            nfo_file.write_all(movie_nfo.as_bytes())?;
            info!(ind: 2, "Generated {}", nfo_path.to_string_lossy());
        }
    }

    if let Some(poster_url) = poster_url
        .as_deref()
        .filter(|_| force || !poster_path.exists())
        && let Err(error) = download_poster(poster_url, &poster_path).await
    {
        warn!(
            ind: 2,
            "Could not download poster to {}: {}",
            poster_path.to_string_lossy(),
            error
        );
    }
    Ok(())
}

fn warn_if_non_theatrical_platform(platform: &str) {
    if platform.trim() != "剧场版" {
        warn!(
            "Bangumi subject platform is {:?}, not \"剧场版\"; continuing because --movie-source explicitly selected movie mode",
            platform
        );
    }
}

fn movie_nfo_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = HashSet::from([path.join(MOVIE_NFO_NAME)]);
    for entry in WalkDir::new(path).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.file_type().is_file() && is_video_file(entry.path()) {
            paths.insert(entry.path().with_extension("nfo"));
        }
    }
    let mut paths: Vec<PathBuf> = paths.into_iter().collect();
    paths.sort();
    Ok(paths)
}

async fn download_poster(url: &str, destination: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("request poster {}", url))?;
    if !response.status().is_success() {
        bail!("poster request returned {}", response.status());
    }
    let bytes = response.bytes().await.with_context(|| "read poster body")?;
    std::fs::write(destination, bytes)?;
    info!(ind: 2, "Downloaded {}", destination.to_string_lossy());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn explicit_movie_mode_accepts_non_theatrical_platform_labels() {
        warn_if_non_theatrical_platform("剧场版");
        warn_if_non_theatrical_platform("OVA");
        warn_if_non_theatrical_platform("TV");
    }

    #[test]
    fn creates_folder_and_video_specific_nfo_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dantalian-movie-{nonce}"));
        std::fs::create_dir_all(&dir).unwrap();
        File::create(dir.join("电影 (2001).mkv")).unwrap();
        File::create(dir.join("说明.txt")).unwrap();

        let paths = movie_nfo_paths(&dir).unwrap();
        assert_eq!(
            paths,
            vec![dir.join("movie.nfo"), dir.join("电影 (2001).nfo")]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
