use clap::{Parser, ValueHint, crate_authors, crate_description, crate_version};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author=crate_authors!(), version=crate_version!(), about=crate_description!())]
pub struct Opts {
    /// show more information
    #[clap(short, long)]
    pub verbose: bool,
    /// anime source folder. can be used multiple times to decide multi source
    #[clap(short, long, required = false, value_hint=ValueHint::DirPath)]
    pub source: Vec<PathBuf>,
    /// movies source folder. can be used multiple times to decide multi source
    #[clap(short, long, required = false, value_hint=ValueHint::DirPath)]
    pub movie_source: Vec<PathBuf>,
    /// movie source folders scraped with TMDB
    #[clap(long, required = false, value_hint=ValueHint::DirPath)]
    pub tmdb_movie_source: Vec<PathBuf>,
    /// TV source folders scraped with TMDB
    #[clap(long, required = false, value_hint=ValueHint::DirPath)]
    pub tmdb_tv_source: Vec<PathBuf>,
    /// paths which you want to force re-generate
    #[clap(long, required = false)]
    pub force: Vec<String>,
    /// force re-generate all nfo files for all anime
    #[clap(long)]
    pub force_all: bool,
    /// use your personal token to access more subject. get one from https://next.bgm.tv/demo/access-token/create.
    #[clap(long)]
    pub access_token: Option<String>,
    /// TMDB API Read Access Token
    #[clap(long)]
    pub tmdb_token: Option<String>,
    #[clap(subcommand)]
    pub subcmd: Option<SubCmd>,
}

#[derive(Parser)]
pub enum SubCmd {
    Bgm(BgmCmd),
    Tmdb(TmdbCmd),
}

/// CLI tools for TMDB APIs
#[derive(Parser)]
pub struct TmdbCmd {
    #[clap(subcommand)]
    pub subcmd: TmdbSubCmd,
}

#[derive(Parser)]
pub enum TmdbSubCmd {
    /// search movies in TMDB
    SearchMovie(TmdbSearchOpt),
    /// search TV shows in TMDB
    SearchTv(TmdbSearchOpt),
    /// get a movie by TMDB id
    GetMovie(TmdbGetOpt),
    /// get a TV show by TMDB id
    GetTv(TmdbGetOpt),
}

#[derive(Parser)]
pub struct TmdbSearchOpt {
    /// search keywords
    pub keyword: Vec<String>,
    /// release year or first air year
    #[clap(long)]
    pub year: Option<u32>,
    /// metadata language
    #[clap(long, default_value = "zh-CN")]
    pub language: String,
}

#[derive(Parser)]
pub struct TmdbGetOpt {
    /// TMDB id
    pub id: u32,
    /// metadata language
    #[clap(long, default_value = "zh-CN")]
    pub language: String,
}

/// cli tools to play with bangumi apis
#[derive(Parser)]
pub struct BgmCmd {
    #[clap(subcommand)]
    pub subcmd: BgmSubCmd,
}

#[derive(Parser)]
pub struct BgmCmdOpt {
    /// show more information
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Parser)]
pub enum BgmSubCmd {
    /// search subject in bangumi
    Search(BgmSearchOpt),
    /// try get subject info by id
    Get(BgmGetSubjectOpt),
    /// try get episode info by subject id
    GetEp(BgmGetSubjectEpsOpt),
}

#[derive(Parser)]
pub struct BgmSearchOpt {
    /// search keywords.
    pub keyword: Vec<String>,
}

#[derive(Parser)]
pub struct BgmGetSubjectOpt {
    /// subject id. can get from search.
    pub id: u32,
    /// doesn't get person(staff) infomation.
    #[clap(long)]
    pub no_persons: bool,
    /// doesn't get characters infomation.
    #[clap(long)]
    pub no_characters: bool,
}

#[derive(Parser)]
pub struct BgmGetSubjectEpsOpt {
    #[clap(help = "subject id")]
    pub id: u32,
}
