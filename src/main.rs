use anyhow::Result;
use clap::Parser;
use dantalian::bangumi;
use dantalian::dantalian::{dantalian, dantalian_movie, dantalian_tmdb_movie, dantalian_tmdb_tv};
use dantalian::tmdb;
use dantalian::{info, logger::Logger};
use log::set_logger;
use options::{BgmCmd, BgmSubCmd, Opts, SubCmd, TmdbCmd, TmdbSubCmd};
use std::collections::HashSet;
use std::iter::FromIterator;

mod options;

#[tokio::main]
async fn main() -> Result<()> {
    let Opts {
        access_token,
        tmdb_token,
        verbose,
        subcmd,
        force_all,
        force,
        source,
        movie_source,
        tmdb_movie_source,
        tmdb_tv_source,
    } = Opts::parse();
    if let Some(access_token) = access_token {
        bangumi::set_access_token(access_token);
    }
    if let Some(tmdb_token) = tmdb_token {
        tmdb::set_access_token(tmdb_token);
    }
    match verbose {
        true => set_logger(Logger::init(log::LevelFilter::Trace)).unwrap(),
        false => set_logger(Logger::init(log::LevelFilter::Info)).unwrap(),
    }
    match subcmd {
        None => {
            let force: HashSet<String> = HashSet::from_iter(force);
            let is_force = |path| force_all || force.contains(&path);
            for source in source {
                dantalian(&source, &is_force).await?;
            }
            for movie_source in movie_source {
                dantalian_movie(&movie_source, &is_force).await?;
            }
            for movie_source in tmdb_movie_source {
                dantalian_tmdb_movie(&movie_source, &is_force).await?;
            }
            for tv_source in tmdb_tv_source {
                dantalian_tmdb_tv(&tv_source, &is_force).await?;
            }
            Ok(())
        }
        Some(subcmd) => match subcmd {
            SubCmd::Bgm(sub_opts) => bgm_cmd(sub_opts).await,
            SubCmd::Tmdb(sub_opts) => tmdb_cmd(sub_opts).await,
        },
    }
}

async fn tmdb_cmd(opts: TmdbCmd) -> Result<()> {
    match opts.subcmd {
        TmdbSubCmd::SearchMovie(search_opts) => {
            let keyword = search_opts.keyword.join(" ");
            let response =
                tmdb::search_movies(&keyword, search_opts.year, &search_opts.language).await?;
            info!("found {} result(s):\n", response.total_results);
            for item in response.results {
                info!("{}", item);
            }
            Ok(())
        }
        TmdbSubCmd::SearchTv(search_opts) => {
            let keyword = search_opts.keyword.join(" ");
            let response =
                tmdb::search_tv(&keyword, search_opts.year, &search_opts.language).await?;
            info!("found {} result(s):\n", response.total_results);
            for item in response.results {
                info!("{}", item);
            }
            Ok(())
        }
        TmdbSubCmd::GetMovie(get_opts) => {
            info!(
                "{:#?}",
                tmdb::get_movie(get_opts.id, &get_opts.language).await?
            );
            Ok(())
        }
        TmdbSubCmd::GetTv(get_opts) => {
            info!(
                "{:#?}",
                tmdb::get_tv(get_opts.id, &get_opts.language).await?
            );
            Ok(())
        }
    }
}

async fn bgm_cmd(opts: BgmCmd) -> Result<()> {
    match opts.subcmd {
        BgmSubCmd::Search(search_opts) => {
            let keyword = search_opts.keyword.join(" ");
            let res = bangumi::search_anime(&keyword).await?;
            info!("found {} result(s):\n", res.data.len());
            for item in res.data {
                info!("{:>1}", item);
            }
            Ok(())
        }
        BgmSubCmd::Get(get_opts) => {
            let subject = bangumi::get_subject(get_opts.id).await?;
            info!("{}", &subject);
            if !get_opts.no_persons {
                let persons = bangumi::get_subject_persons(get_opts.id).await?;
                info!("{}", persons);
            }
            if !get_opts.no_characters {
                let characters = bangumi::get_subject_characters(get_opts.id).await?;
                info!("{}", characters);
            }
            Ok(())
        }
        BgmSubCmd::GetEp(get_opts) => {
            let res = bangumi::get_subject_episodes(get_opts.id).await?;
            info!("{}", &res);
            Ok(())
        }
    }
}
