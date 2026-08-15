//! Loading and validating a language pack.
//!
//! A malformed pack stops the server from starting. That is on purpose: a typo
//! in a topic id should be a loud failure at boot, not a silently missing topic
//! in the middle of a study session.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::Level;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("cannot read pack file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("pack `{language}` is invalid: {message}")]
    Invalid { language: String, message: String },
}

impl PackError {
    fn invalid(language: &str, message: impl Into<String>) -> Self {
        PackError::Invalid {
            language: language.to_string(),
            message: message.into(),
        }
    }
}

/// Whether a topic has ever been studied before lacuna existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TopicStatus {
    /// Never studied. The scheduler introduces these as new.
    New,
    /// Studied elsewhere already, so it starts as a review rather than a lesson.
    #[default]
    Known,
}

#[derive(Debug, Clone, Serialize)]
pub struct Topic {
    pub id: String,
    pub cefr: Level,
    pub stage: u32,
    pub category: String,
    pub title: String,
    pub goal: String,
    pub status: TopicStatus,
}

/// The raw shape of one `[[topic]]` entry in `topics.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicDef {
    id: String,
    cefr: Level,
    stage: u32,
    category: String,
    title: String,
    goal: String,
    #[serde(default)]
    status: TopicStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackFile {
    #[serde(default)]
    topic: Vec<TopicDef>,
}

#[derive(Debug, Clone)]
pub struct Pack {
    pub language: String,
    /// Sorted by level, then by teaching stage inside that level.
    pub topics: Vec<Topic>,
}

impl Pack {
    /// Load `<root>/<language>/topics.toml`.
    pub fn load(root: &Path, language: &str) -> Result<Pack, PackError> {
        let path = root.join(language).join("topics.toml");
        let raw = std::fs::read_to_string(&path).map_err(|source| PackError::Read {
            path: path.clone(),
            source,
        })?;
        let parsed: PackFile = toml::from_str(&raw).map_err(|source| PackError::Parse {
            path: path.clone(),
            source,
        })?;
        Pack::from_defs(language, parsed.topic)
    }

    fn from_defs(language: &str, defs: Vec<TopicDef>) -> Result<Pack, PackError> {
        if defs.is_empty() {
            return Err(PackError::invalid(language, "pack contains no topics"));
        }

        let mut seen_ids: HashSet<&str> = HashSet::new();
        let mut seen_stages: HashSet<(Level, u32)> = HashSet::new();

        for def in &defs {
            validate_id(language, &def.id)?;
            if !seen_ids.insert(def.id.as_str()) {
                return Err(PackError::invalid(
                    language,
                    format!("duplicate topic id `{}`", def.id),
                ));
            }
            if !seen_stages.insert((def.cefr, def.stage)) {
                return Err(PackError::invalid(
                    language,
                    format!("two topics share stage {} in {}", def.stage, def.cefr),
                ));
            }
            if def.stage == 0 {
                return Err(PackError::invalid(
                    language,
                    format!("topic `{}` has stage 0, stages start at 1", def.id),
                ));
            }
            for (field, value) in [
                ("category", &def.category),
                ("title", &def.title),
                ("goal", &def.goal),
            ] {
                if value.trim().is_empty() {
                    return Err(PackError::invalid(
                        language,
                        format!("topic `{}` has an empty {field}", def.id),
                    ));
                }
            }
        }

        // Stages must be a gapless 1..n run inside each level, otherwise the
        // teaching order has a hole in it that nobody noticed.
        for level in Level::ALL {
            let mut stages: Vec<u32> = defs
                .iter()
                .filter(|d| d.cefr == level)
                .map(|d| d.stage)
                .collect();
            if stages.is_empty() {
                continue;
            }
            stages.sort_unstable();
            for (index, stage) in stages.iter().enumerate() {
                let expected = index as u32 + 1;
                if *stage != expected {
                    return Err(PackError::invalid(
                        language,
                        format!("{level} stages must run 1..{}, found {stage} where {expected} was expected", stages.len()),
                    ));
                }
            }
        }

        let mut topics: Vec<Topic> = defs
            .into_iter()
            .map(|d| Topic {
                id: d.id,
                cefr: d.cefr,
                stage: d.stage,
                category: d.category,
                title: d.title,
                goal: d.goal,
                status: d.status,
            })
            .collect();
        topics.sort_by(|a, b| a.cefr.cmp(&b.cefr).then(a.stage.cmp(&b.stage)));

        Ok(Pack {
            language: language.to_string(),
            topics,
        })
    }

    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|t| t.id == id)
    }
}

/// Topic ids look like `cases.dative_prepositions`: one dot, lowercase ascii.
fn validate_id(language: &str, id: &str) -> Result<(), PackError> {
    let Some((head, tail)) = id.split_once('.') else {
        return Err(PackError::invalid(
            language,
            format!("topic id `{id}` must look like `category.topic`"),
        ));
    };
    if head.is_empty() || tail.is_empty() {
        return Err(PackError::invalid(
            language,
            format!("topic id `{id}` has an empty half"),
        ));
    }
    if tail.contains('.') {
        return Err(PackError::invalid(
            language,
            format!("topic id `{id}` has more than one dot"),
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
    {
        return Err(PackError::invalid(
            language,
            format!("topic id `{id}` must be lowercase ascii, digits, underscore or dot"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str, cefr: Level, stage: u32) -> TopicDef {
        TopicDef {
            id: id.to_string(),
            cefr,
            stage,
            category: "Cases".to_string(),
            title: "Dative case".to_string(),
            goal: "Practice when to use the dative".to_string(),
            status: TopicStatus::Known,
        }
    }

    #[test]
    fn sorts_by_level_then_stage() {
        let pack = Pack::from_defs(
            "de",
            vec![
                def("cases.b", Level::A2, 2),
                def("cases.a", Level::A2, 1),
                def("articles.a", Level::A1, 1),
            ],
        )
        .unwrap();
        let ids: Vec<&str> = pack.topics.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["articles.a", "cases.a", "cases.b"]);
    }

    #[test]
    fn rejects_duplicate_ids() {
        let err = Pack::from_defs("de", vec![def("cases.a", Level::A1, 1), def("cases.a", Level::A1, 2)])
            .unwrap_err();
        assert!(err.to_string().contains("duplicate topic id"));
    }

    #[test]
    fn rejects_duplicate_stage_in_one_level() {
        let err = Pack::from_defs("de", vec![def("cases.a", Level::A1, 1), def("cases.b", Level::A1, 1)])
            .unwrap_err();
        assert!(err.to_string().contains("share stage"));
    }

    #[test]
    fn rejects_gap_in_stages() {
        let err = Pack::from_defs("de", vec![def("cases.a", Level::A1, 1), def("cases.b", Level::A1, 3)])
            .unwrap_err();
        assert!(err.to_string().contains("stages must run"));
    }

    #[test]
    fn same_stage_in_different_levels_is_fine() {
        let pack = Pack::from_defs("de", vec![def("cases.a", Level::A1, 1), def("cases.b", Level::A2, 1)]);
        assert!(pack.is_ok());
    }

    #[test]
    fn rejects_bad_ids() {
        for bad in ["dative", "cases.dative.extra", "Cases.dative", "cases."] {
            let err = Pack::from_defs("de", vec![def(bad, Level::A1, 1)]).unwrap_err();
            assert!(err.to_string().contains("topic id"), "accepted `{bad}`");
        }
    }

    #[test]
    fn rejects_empty_pack() {
        assert!(Pack::from_defs("de", vec![]).is_err());
    }

    #[test]
    fn loads_the_real_german_pack() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs");
        let pack = Pack::load(&root, "de").expect("the shipped German pack must load");
        assert_eq!(pack.topics.len(), 43);
        assert_eq!(pack.topics[0].id, "articles.definite");
        assert!(pack.topic("cases.dative").is_some());
    }
}
