//! Closed sets the rest of the program is built on.
//!
//! Everything here is deliberately an enum. A grammar pack that names a level or
//! an error kind lacuna does not know about must fail to load, not produce a
//! broken sheet three weeks later.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// CEFR level, as labelled by the source curriculum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub enum Level {
    A1,
    A2,
    B1,
    B2,
    C1,
    C2,
}

impl Level {
    pub const ALL: [Level; 6] = [
        Level::A1,
        Level::A2,
        Level::B1,
        Level::B2,
        Level::C1,
        Level::C2,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Level::A1 => "A1",
            Level::A2 => "A2",
            Level::B1 => "B1",
            Level::B2 => "B2",
            Level::C1 => "C1",
            Level::C2 => "C2",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Level {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "A1" => Ok(Level::A1),
            "A2" => Ok(Level::A2),
            "B1" => Ok(Level::B1),
            "B2" => Ok(Level::B2),
            "C1" => Ok(Level::C1),
            "C2" => Ok(Level::C2),
            other => Err(format!("unknown CEFR level `{other}`")),
        }
    }
}

/// How well a whole sheet went. Maps one to one onto the FSRS buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../web/src/lib/types/", rename_all = "lowercase")]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    /// The value FSRS expects: 1 to 4.
    pub fn as_fsrs(self) -> u32 {
        match self {
            Rating::Again => 1,
            Rating::Hard => 2,
            Rating::Good => 3,
            Rating::Easy => 4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Rating::Again => "again",
            Rating::Hard => "hard",
            Rating::Good => "good",
            Rating::Easy => "easy",
        }
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Rating {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "again" => Ok(Rating::Again),
            "hard" => Ok(Rating::Hard),
            "good" => Ok(Rating::Good),
            "easy" => Ok(Rating::Easy),
            other => Err(format!("unknown rating `{other}`")),
        }
    }
}

/// The kind of mistake a blank can capture.
///
/// These are produced at generation time, while the generator still knows what
/// the blank was testing. They are never guessed from a wrong answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    /// Grammatical case: nominative, accusative, dative, genitive.
    Case,
    /// Grammatical gender of the noun involved.
    Gender,
    /// Singular or plural.
    Number,
    /// What forced the case or form, for example a preposition or a verb.
    Trigger,
    /// Verb form: tense, person, participle.
    Form,
    /// Adjective or article declension pattern.
    Declension,
    /// Position of a word in the sentence.
    WordOrder,
    /// Spelling only, the grammar was right.
    Spelling,
    /// Capitalisation only, the letters were right.
    Capitalisation,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Case => "case",
            ErrorKind::Gender => "gender",
            ErrorKind::Number => "number",
            ErrorKind::Trigger => "trigger",
            ErrorKind::Form => "form",
            ErrorKind::Declension => "declension",
            ErrorKind::WordOrder => "word_order",
            ErrorKind::Spelling => "spelling",
            ErrorKind::Capitalisation => "capitalisation",
        }
    }
}

impl FromStr for ErrorKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "case" => Ok(ErrorKind::Case),
            "gender" => Ok(ErrorKind::Gender),
            "number" => Ok(ErrorKind::Number),
            "trigger" => Ok(ErrorKind::Trigger),
            "form" => Ok(ErrorKind::Form),
            "declension" => Ok(ErrorKind::Declension),
            "word_order" => Ok(ErrorKind::WordOrder),
            "spelling" => Ok(ErrorKind::Spelling),
            "capitalisation" => Ok(ErrorKind::Capitalisation),
            other => Err(format!("unknown error kind `{other}`")),
        }
    }
}

/// A tag written as `kind:detail`, for example `case:dative` or
/// `trigger:preposition_mit`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorTag {
    pub kind: ErrorKind,
    pub detail: String,
}

impl ErrorTag {
    pub fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ErrorTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.detail)
    }
}

impl FromStr for ErrorTag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, detail) = s
            .split_once(':')
            .ok_or_else(|| format!("tag `{s}` is not in `kind:detail` form"))?;
        let kind: ErrorKind = kind.parse()?;
        if detail.is_empty() {
            return Err(format!("tag `{s}` has an empty detail"));
        }
        if !detail
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(format!(
                "tag detail `{detail}` must be lowercase ascii, digits or underscore"
            ));
        }
        Ok(ErrorTag {
            kind,
            detail: detail.to_string(),
        })
    }
}

impl Serialize for ErrorTag {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ErrorTag {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_round_trip() {
        for level in Level::ALL {
            assert_eq!(level.as_str().parse::<Level>().unwrap(), level);
        }
    }

    #[test]
    fn ratings_map_to_fsrs_buttons() {
        assert_eq!(Rating::Again.as_fsrs(), 1);
        assert_eq!(Rating::Hard.as_fsrs(), 2);
        assert_eq!(Rating::Good.as_fsrs(), 3);
        assert_eq!(Rating::Easy.as_fsrs(), 4);
    }

    #[test]
    fn tags_round_trip() {
        let tag: ErrorTag = "trigger:preposition_mit".parse().unwrap();
        assert_eq!(tag.kind, ErrorKind::Trigger);
        assert_eq!(tag.detail, "preposition_mit");
        assert_eq!(tag.to_string(), "trigger:preposition_mit");
    }

    #[test]
    fn bad_tags_are_rejected() {
        assert!("dative".parse::<ErrorTag>().is_err());
        assert!("mood:dative".parse::<ErrorTag>().is_err());
        assert!("case:".parse::<ErrorTag>().is_err());
        assert!("case:Dativ".parse::<ErrorTag>().is_err());
    }
}
