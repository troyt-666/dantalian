use serde::Serialize;
use std::rc::Rc;

use crate::bangumi::{BgmAnime, EpisodeType, PersonType};

pub const TVSHOW_NFO_NAME: &str = "tvshow.nfo";

// TVShow file is for overall show information.
// TVShow file name must actually be tvshow.nfo.
// This file must be tv show's folder's root.
#[derive(Serialize, Debug)]
pub struct TVShow {
    pub uid: u32,
    pub title: String,
    pub original_title: String,
    pub rating_value: f64,
    pub rating_votes: u32,
    pub has_sp: bool,
    pub eps_count: Option<u32>,
    pub plot: String,
    pub poster: Option<String>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub premiered: String,
    pub status: Option<String>,
    pub studio: Option<String>,
    pub actors: Rc<[Actor]>,
}

#[derive(Serialize, Debug)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub order: u32,
    pub thumb: String,
}

pub const TVSHOW_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<tvshow>
    <title>{title}</title>
    <originaltitle>{original_title}</originaltitle>
    <ratings>
        <rating name="bangumi" max="10" default="true">
            <value>{rating_value}</value>
            <votes>{rating_votes}</votes>
        </rating>
    </ratings>
    <season>{{ if has_sp }}2{{ else }}1{{ endif }}</season>
    {{ if eps_count }}<episode>{eps_count}</episode>{{ endif }}
    <plot>{plot}</plot>
    {{ if poster }}<thumb aspect="poster" preview="{poster}">{poster}</thumb>{{ endif }}
    <uniqueid type="bangumi" default="true">{uid}</uniqueid>{{ for g in genres }}
    <genre>{g}</genre>{{ endfor }}{{ for t in tags }}
    <tag>{t}</tag>{{ endfor }}
    <premiered>{premiered}</premiered>{{ if status }}
    <status>{status}</status>{{ endif }}{{ if studio }}
    <studio>{studio}</studio>{{ endif }}{{ for a in actors }}
    <actor>
        <name>{a.name}</name>
        <role>{a.role}</role>
        <order>{a.order}</order>
        <thumb>{a.thumb}</thumb>
    </actor>{{ endfor }}
</tvshow>
"#;

// Episode file is for single episode, this file must
// place alongside of media file, and use same file name.
#[derive(Serialize, Debug)]
pub struct Episode {
    pub uid: u32,
    pub title: String,
    pub original_title: String,
    pub show_title: String,
    pub rating_value: Option<f64>,
    pub rating_votes: Option<u32>,
    pub ep_index: String,
    pub is_sp: bool,
    pub plot: String,
    pub directors: Rc<[String]>,
    pub credits: Rc<[String]>,
    pub premiered: String,
    pub status: Option<String>,
    pub aired: Option<String>,
    pub studio: Option<String>,
    pub actors: Rc<[Actor]>,
}

pub const EPISODE_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<episodedetails>
    <title>{title}</title>
    <originaltitle>{original_title}</originaltitle>
    <showtitle>{show_title}</showtitle>{{ if rating_value }}
    <ratings>
        <rating name="bangumi" max="10" default="true">
            <value>{rating_value}</value>
            {{ if rating_votes }}<votes>{rating_votes}</votes>{{ endif }}
        </rating>
    </ratings>{{ endif }}
    <season>{{ if is_sp }}0{{ else }}1{{ endif }}</season>
    <episode>{ep_index}</episode>
    <plot>{plot}</plot>
    <uniqueid type="bangumi" default="true">{uid}</uniqueid>{{ for c in credits }}
    <credits>{c}</credits>{{ endfor }}{{ for d in directors }}
    <director>{d}</director>{{ endfor }}
    <premiered>{premiered}</premiered>{{ if status }}
    <status>{status}</status>
    {{ endif }}<aired>{aired}</aired>{{ if studio }}
    <studio>{studio}</studio>{{ endif }}{{ for a in actors }}
    <actor>
        <name>{a.name}</name>
        <role>{a.role}</role>
        <order>{a.order}</order>
        <thumb>{a.thumb}</thumb>
    </actor>{{ endfor }}
</episodedetails>
"#;

pub const MOVIE_NFO_NAME: &str = "movie.nfo";

#[derive(Serialize, Debug)]
pub struct Movie {
    pub uid: u32,
    pub title: String,
    pub original_title: String,
    pub rating_value: f64,
    pub rating_votes: u32,
    pub plot: String,
    pub poster: Option<String>,
    pub year: Option<u32>,
    pub runtime: Option<u32>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub premiered: String,
    pub status: Option<String>,
    pub studio: Option<String>,
    pub directors: Vec<String>,
    pub credits: Vec<String>,
    pub actors: Vec<Actor>,
}

impl Movie {
    pub fn from_bgm(bgm_data: BgmAnime) -> Self {
        let BgmAnime {
            subject,
            episodes,
            persons,
            characters,
        } = bgm_data;
        let mut actors: Vec<Actor> = Vec::new();
        for character in characters {
            for actor in character.actors {
                actors.push(Actor {
                    name: actor.name,
                    role: character.name.clone(),
                    order: actors.len() as u32,
                    thumb: actor.images.map_or(String::new(), |images| images.large),
                });
            }
        }

        let mut directors = Vec::new();
        let mut credits = Vec::new();
        let mut studio: Option<(u8, String)> = None;
        for person in persons {
            if has_role(&person.relation, &["导演", "監督"]) {
                directors.push(person.name.clone());
            }
            if has_role(&person.relation, &["脚本", "编剧"]) {
                credits.push(person.name.clone());
            }
            if matches!(person.person_type, PersonType::Company) {
                let priority = if has_role(&person.relation, &["动画制作"]) {
                    2
                } else if has_role(&person.relation, &["制作", "製作"]) {
                    1
                } else {
                    0
                };
                if priority > studio.as_ref().map_or(0, |(current, _)| *current) {
                    studio = Some((priority, person.name));
                }
            }
        }

        let mut tags = Vec::new();
        let mut genres = Vec::new();
        for tag in subject.tags {
            if !tags.contains(&tag.name) {
                tags.push(tag.name.clone());
            }
            if is_genre(&tag.name) && !genres.contains(&tag.name) {
                genres.push(tag.name);
            }
        }

        let runtime = episodes
            .iter()
            .find(|episode| episode.episode_type == EpisodeType::Honpen)
            .and_then(runtime_minutes);
        let year = subject
            .date
            .as_deref()
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok());
        let title = if subject.name_cn.trim().is_empty() {
            subject.name.clone()
        } else {
            subject.name_cn.clone()
        };

        Self {
            uid: subject.id,
            title,
            original_title: subject.name,
            rating_value: subject.rating.score,
            rating_votes: subject.rating.total,
            plot: subject.summary,
            poster: subject.images.map(|img| img.large),
            year,
            runtime,
            genres,
            tags,
            premiered: subject.date.as_deref().unwrap_or("*").to_string(),
            status: None,
            studio: studio.map(|(_, name)| name),
            directors,
            credits,
            actors,
        }
    }
}

fn has_role(relation: &str, expected: &[&str]) -> bool {
    relation
        .split_whitespace()
        .any(|role| expected.contains(&role))
}

fn runtime_minutes(episode: &crate::bangumi::Episode) -> Option<u32> {
    if let Some(seconds) = episode.duration_seconds {
        return Some(seconds.div_ceil(60));
    }
    let duration = episode.duration.trim();
    if let Some(minutes) = duration.strip_suffix('m') {
        return minutes.trim().parse().ok();
    }
    let parts: Vec<u32> = duration
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match parts.as_slice() {
        [hours, minutes, seconds] => Some(hours * 60 + minutes + u32::from(*seconds > 0)),
        [minutes, seconds] => Some(minutes + u32::from(*seconds > 0)),
        _ => None,
    }
}

fn is_genre(tag: &str) -> bool {
    matches!(
        tag,
        "动作"
            | "冒险"
            | "喜剧"
            | "剧情"
            | "家庭"
            | "奇幻"
            | "恐怖"
            | "音乐"
            | "悬疑"
            | "科幻"
            | "运动"
            | "战争"
            | "爱情"
            | "犯罪"
            | "惊悚"
            | "历史"
    )
}

pub const MOVIE_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<movie>
    <title>{title}</title>
    <originaltitle>{original_title}</originaltitle>
    <ratings>
        <rating name="bangumi" max="10" default="true">
            <value>{rating_value}</value>
            <votes>{rating_votes}</votes>
        </rating>
    </ratings>
    <plot>{plot}</plot>
    {{ if poster }}<thumb aspect="poster" preview="{poster}">{poster}</thumb>{{ endif }}
    <uniqueid type="bangumi" default="true">{uid}</uniqueid>{{ for g in genres }}
    <genre>{g}</genre>{{ endfor }}{{ for t in tags }}
    <tag>{t}</tag>{{ endfor }}
    <premiered>{premiered}</premiered>{{ if year }}
    <year>{year}</year>{{ endif }}{{ if runtime }}
    <runtime>{runtime}</runtime>{{ endif }}{{ if status }}
    <status>{status}</status>{{ endif }}{{ if studio }}
    <studio>{studio}</studio>{{ endif }}{{ for c in credits }}
    <credits>{c}</credits>{{ endfor }}{{ for d in directors }}
    <director>{d}</director>{{ endfor }}{{ for a in actors }}
    <actor>
        <name>{a.name}</name>
        <role>{a.role}</role>
        <order>{a.order}</order>
        <thumb>{a.thumb}</thumb>
    </actor>{{ endfor }}
</movie>
"#;

#[cfg(test)]
mod movie_tests {
    use super::*;
    use crate::bangumi::{
        Actor as BgmActor, Character, CharacterImage, Episode as BgmEpisode, Person, PersonCareer,
        Subject, SubjectCollection, SubjectImage, SubjectRating, SubjectRatingCount, SubjectType,
        Tag,
    };

    fn rating() -> SubjectRating {
        SubjectRating {
            rank: 1,
            total: 100,
            score: 8.5,
            count: SubjectRatingCount {
                s1: 0,
                s2: 0,
                s3: 0,
                s4: 0,
                s5: 0,
                s6: 0,
                s7: 0,
                s8: 0,
                s9: 0,
                s10: 0,
            },
        }
    }

    fn image() -> CharacterImage {
        CharacterImage {
            large: "actor-large.jpg".into(),
            medium: "actor-medium.jpg".into(),
            small: "actor-small.jpg".into(),
            grid: "actor-grid.jpg".into(),
        }
    }

    #[test]
    fn movie_uses_bangumi_movie_metadata() {
        let subject = Subject {
            id: 311,
            subject_type: SubjectType::Anime,
            name: "千と千尋の神隠し".into(),
            name_cn: "".into(),
            summary: "剧情".into(),
            nsfw: false,
            date: Some("2001-07-20".into()),
            platform: "剧场版".into(),
            images: Some(SubjectImage {
                large: "poster.jpg".into(),
                common: String::new(),
                medium: String::new(),
                small: String::new(),
                grid: String::new(),
            }),
            eps: Some(1),
            total_episodes: Some(1),
            rating: rating(),
            collection: SubjectCollection {
                wish: 0,
                collect: 0,
                doing: 0,
                on_hold: 0,
                dropped: 0,
            },
            tags: vec![
                Tag {
                    name: "奇幻".into(),
                    count: 10,
                },
                Tag {
                    name: "吉卜力".into(),
                    count: 8,
                },
            ],
        };
        let episodes = vec![BgmEpisode {
            id: 1,
            episode_type: EpisodeType::Honpen,
            ep: Some(1.0),
            sort: 1.0,
            name: String::new(),
            name_cn: String::new(),
            duration: String::new(),
            airdate: "2001-07-20".into(),
            comment: 0,
            desc: String::new(),
            duration_seconds: Some(7501),
        }];
        let persons = vec![Person {
            id: 1,
            images: None,
            person_type: PersonType::Company,
            career: vec![],
            name: "Studio Ghibli".into(),
            relation: "动画制作".into(),
        }];
        let characters = vec![Character {
            id: 1,
            name: "荻野千寻".into(),
            character_type: 1,
            images: None,
            relation: "主角".into(),
            actors: vec![BgmActor {
                id: 2,
                name: "柊瑠美".into(),
                actor_type: PersonType::Person,
                career: vec![PersonCareer::Seiyu],
                short_summary: String::new(),
                locked: false,
                images: Some(image()),
            }],
        }];

        let movie = Movie::from_bgm(BgmAnime {
            subject,
            episodes,
            persons,
            characters,
        });
        assert_eq!(movie.title, "千と千尋の神隠し");
        assert_eq!(movie.year, Some(2001));
        assert_eq!(movie.runtime, Some(126));
        assert_eq!(movie.genres, vec!["奇幻"]);
        assert_eq!(movie.tags, vec!["奇幻", "吉卜力"]);
        assert_eq!(movie.studio.as_deref(), Some("Studio Ghibli"));
        assert_eq!(movie.actors[0].name, "柊瑠美");
        assert_eq!(movie.actors[0].role, "荻野千寻");
    }

    #[test]
    fn movie_does_not_treat_assistant_director_as_director() {
        assert!(has_role("原作 脚本 导演", &["导演", "監督"]));
        assert!(!has_role("助理导演", &["导演", "監督"]));
    }

    #[test]
    fn movie_prefers_animation_studio_over_generic_producer() {
        let mut bgm = BgmAnime {
            subject: Subject {
                id: 1,
                subject_type: SubjectType::Anime,
                name: "Movie".into(),
                name_cn: String::new(),
                summary: String::new(),
                nsfw: false,
                date: Some("2000-01-01".into()),
                platform: "剧场版".into(),
                images: None,
                eps: Some(1),
                total_episodes: Some(1),
                rating: rating(),
                collection: SubjectCollection {
                    wish: 0,
                    collect: 0,
                    doing: 0,
                    on_hold: 0,
                    dropped: 0,
                },
                tags: vec![],
            },
            episodes: vec![],
            persons: vec![],
            characters: vec![],
        };
        bgm.persons.push(Person {
            id: 1,
            images: None,
            person_type: PersonType::Company,
            career: vec![],
            name: "Publisher".into(),
            relation: "制作".into(),
        });
        bgm.persons.push(Person {
            id: 2,
            images: None,
            person_type: PersonType::Company,
            career: vec![],
            name: "Animation Studio".into(),
            relation: "动画制作".into(),
        });

        assert_eq!(
            Movie::from_bgm(bgm).studio.as_deref(),
            Some("Animation Studio")
        );
    }
}
