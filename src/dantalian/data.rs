use crate::bangumi::{BgmAnime, EpisodeType};
use crate::nfogen::{Actor, Episode, Rating, TVShow, UniqueId};
use std::rc::Rc;

// AnimeData store data for generator nfo files.
#[derive(Debug)]
pub struct AnimeData {
    pub tvshow: TVShow,
    pub episodes: Vec<Episode>,
}

impl AnimeData {
    pub fn find_episode(&self, index: &str, is_sp: bool) -> Option<&Episode> {
        self.episodes
            .iter()
            .find(|&ep| ep.ep_index == index && ep.is_sp == is_sp)
    }
}

impl From<BgmAnime> for AnimeData {
    fn from(bgm_data: BgmAnime) -> Self {
        let BgmAnime {
            subject,
            episodes,
            persons,
            characters,
        } = bgm_data;
        let mut data = AnimeData {
            episodes: Vec::new(),
            tvshow: TVShow {
                title: subject.name_cn,
                original_title: subject.name,
                ratings: vec![Rating {
                    name: "bangumi".into(),
                    max: 10,
                    is_default: true,
                    value: subject.rating.score,
                    votes: Some(subject.rating.total),
                }],
                unique_ids: vec![UniqueId {
                    kind: "bangumi".into(),
                    is_default: true,
                    value: subject.id.to_string(),
                }],
                season_count: 1,
                eps_count: subject.total_episodes,
                plot: subject.summary,
                poster: subject.images.map(|img| img.large),
                fanart: None,
                genres: vec![],
                tags: vec![],
                premiered: subject.date.as_deref().unwrap_or("*").to_string(),
                status: None,
                studio: None,
                actors: Rc::from(Vec::new()),
            },
        };

        let mut actors: Vec<Actor> = Vec::new();
        for character in characters {
            if character.actors.is_empty() {
                actors.push(Actor {
                    name: character.name,
                    role: String::from("N/A"),
                    order: actors.len() as u32,
                    thumb: character.images.map_or(String::from(""), |ci| ci.large),
                });
            } else {
                for actor in character.actors {
                    actors.push(Actor {
                        name: character.name.clone(),
                        role: actor.name,
                        order: actors.len() as u32,
                        thumb: character
                            .images
                            .as_ref()
                            .map_or(String::from(""), |ci| ci.large.clone()),
                    });
                }
            }
        }
        data.tvshow.actors = Rc::from(actors);

        let mut credits: Vec<String> = Vec::new();
        let mut directors: Vec<String> = Vec::new();
        // staff
        for person in persons {
            if person.relation == "导演" {
                directors.push(person.name);
            } else if person.relation == "脚本" {
                credits.push(person.name);
            }
        }
        let rc_directors = Rc::from(directors);
        let rc_credits = Rc::from(credits);

        for be in episodes {
            if !be.is_empty() {
                let is_sp = be.episode_type == EpisodeType::Sp;
                if is_sp {
                    data.tvshow.season_count = 2;
                }
                data.episodes.push(Episode {
                    title: be.name_cn,
                    original_title: be.name,
                    show_title: String::from(&data.tvshow.title),
                    ratings: vec![],
                    unique_ids: vec![UniqueId {
                        kind: "bangumi".into(),
                        is_default: true,
                        value: be.id.to_string(),
                    }],
                    season: if is_sp { 0 } else { 1 },
                    ep_index: format!("{}", be.sort),
                    is_sp,
                    plot: be.desc,
                    thumb: None,
                    runtime: be.duration_seconds.map(|seconds| seconds.div_ceil(60)),
                    directors: Rc::clone(&rc_directors),
                    credits: Rc::clone(&rc_credits),
                    premiered: String::from(&data.tvshow.premiered),
                    // New bangumi api has no status.
                    status: None,
                    aired: Some(be.airdate),
                    studio: None,
                    actors: Rc::clone(&data.tvshow.actors),
                })
            }
        }
        data
    }
}
