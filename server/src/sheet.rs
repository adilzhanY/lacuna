//! The exercise unit: one topic, twenty items.
//!
//! A sheet is validated before it is ever stored. An invalid sheet is thrown
//! away and regenerated, never repaired at read time.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::ErrorTag;

/// Every sheet holds exactly this many items.
pub const ITEMS_PER_SHEET: usize = 20;

#[derive(Debug, Error, PartialEq)]
pub enum SheetError {
    #[error("sheet must hold exactly {ITEMS_PER_SHEET} items, found {0}")]
    WrongItemCount(usize),
    #[error("item {item} has no blank")]
    ItemWithoutBlank { item: u32 },
    #[error("item {item} has no visible text")]
    ItemWithoutText { item: u32 },
    #[error("blank `{blank}` has an empty accept list")]
    EmptyAcceptList { blank: String },
    #[error("blank `{blank}` accepts an empty answer")]
    EmptyAnswer { blank: String },
    #[error("duplicate blank id `{blank}`")]
    DuplicateBlankId { blank: String },
    #[error("item {item} leaks the answer `{answer}` in its visible text")]
    LeakedAnswer { item: u32, answer: String },
    #[error("item numbers must run 1..{ITEMS_PER_SHEET}, found {found} where {expected} was expected")]
    BadItemNumber { found: u32, expected: u32 },
}

/// One gap in a sentence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blank {
    /// Unique inside the sheet, for example `3a`.
    pub id: String,
    /// Every answer counted as correct. Always a list, never a single string,
    /// because "also accept" has to be able to extend it forever.
    pub accept: Vec<String>,
    /// What this blank tests, written by the generator, not inferred later.
    #[serde(default)]
    pub tags: Vec<ErrorTag>,
}

impl Blank {
    /// The answer shown to the user when they got it wrong.
    pub fn canonical(&self) -> &str {
        self.accept.first().map(String::as_str).unwrap_or_default()
    }
}

/// A sentence is a run of visible text and gaps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Segment {
    Text { text: String },
    Blank(Blank),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    /// 1 to 20, in order.
    pub n: u32,
    pub segments: Vec<Segment>,
    /// The infinitives or cue words shown in brackets after the sentence.
    #[serde(default)]
    pub hint: Option<String>,
}

impl Item {
    pub fn blanks(&self) -> impl Iterator<Item = &Blank> {
        self.segments.iter().filter_map(|s| match s {
            Segment::Blank(b) => Some(b),
            Segment::Text { .. } => None,
        })
    }

    pub fn blanks_mut(&mut self) -> impl Iterator<Item = &mut Blank> {
        self.segments.iter_mut().filter_map(|s| match s {
            Segment::Blank(b) => Some(b),
            Segment::Text { .. } => None,
        })
    }

    /// True when this blank is the first thing in the sentence, which is the one
    /// case where a capital letter on a normally lowercase word is fine.
    pub fn is_sentence_initial(&self, blank_id: &str) -> bool {
        for segment in &self.segments {
            match segment {
                Segment::Text { text } if text.trim().is_empty() => continue,
                Segment::Text { .. } => return false,
                Segment::Blank(b) => return b.id == blank_id,
            }
        }
        false
    }

    fn visible_text(&self) -> String {
        self.segments
            .iter()
            .filter_map(|s| match s {
                Segment::Text { text } => Some(text.as_str()),
                Segment::Blank(_) => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sheet {
    pub topic_id: String,
    pub language: String,
    pub items: Vec<Item>,
}

impl Sheet {
    /// Every rule a generated sheet has to satisfy before it can be stored.
    pub fn validate(&self) -> Result<(), SheetError> {
        if self.items.len() != ITEMS_PER_SHEET {
            return Err(SheetError::WrongItemCount(self.items.len()));
        }

        let mut ids: HashSet<&str> = HashSet::new();

        for (index, item) in self.items.iter().enumerate() {
            let expected = index as u32 + 1;
            if item.n != expected {
                return Err(SheetError::BadItemNumber {
                    found: item.n,
                    expected,
                });
            }

            let blanks: Vec<&Blank> = item.blanks().collect();
            if blanks.is_empty() {
                return Err(SheetError::ItemWithoutBlank { item: item.n });
            }
            if item.visible_text().trim().is_empty() {
                return Err(SheetError::ItemWithoutText { item: item.n });
            }

            let visible = item.visible_text().to_lowercase();
            for blank in blanks {
                if !ids.insert(blank.id.as_str()) {
                    return Err(SheetError::DuplicateBlankId {
                        blank: blank.id.clone(),
                    });
                }
                if blank.accept.is_empty() {
                    return Err(SheetError::EmptyAcceptList {
                        blank: blank.id.clone(),
                    });
                }
                for answer in &blank.accept {
                    if answer.trim().is_empty() {
                        return Err(SheetError::EmptyAnswer {
                            blank: blank.id.clone(),
                        });
                    }
                    if leaks(&visible, &answer.to_lowercase()) {
                        return Err(SheetError::LeakedAnswer {
                            item: item.n,
                            answer: answer.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    pub fn blank(&self, blank_id: &str) -> Option<&Blank> {
        self.items.iter().flat_map(Item::blanks).find(|b| b.id == blank_id)
    }

    pub fn item_of(&self, blank_id: &str) -> Option<&Item> {
        self.items
            .iter()
            .find(|i| i.blanks().any(|b| b.id == blank_id))
    }

    /// Add an answer to a blank's accept list. This is what "also accept" does,
    /// and it patches the stored sheet permanently.
    pub fn accept_also(&mut self, blank_id: &str, answer: &str) -> bool {
        let answer = answer.trim();
        if answer.is_empty() {
            return false;
        }
        for item in &mut self.items {
            for blank in item.blanks_mut() {
                if blank.id == blank_id {
                    if blank.accept.iter().any(|a| a == answer) {
                        return false;
                    }
                    blank.accept.push(answer.to_string());
                    return true;
                }
            }
        }
        false
    }
}

/// Whether an answer appears as a whole word in the visible part of the sentence.
///
/// Substring matching alone would be useless here, since "war" sits inside
/// "warten" and would flag a perfectly good item.
fn leaks(visible_lower: &str, answer_lower: &str) -> bool {
    visible_lower
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .any(|word| word == answer_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Segment {
        Segment::Text {
            text: s.to_string(),
        }
    }

    fn blank(id: &str, accept: &[&str]) -> Segment {
        Segment::Blank(Blank {
            id: id.to_string(),
            accept: accept.iter().map(|s| s.to_string()).collect(),
            tags: vec![],
        })
    }

    fn item(n: u32) -> Item {
        Item {
            n,
            segments: vec![
                text("Gestern"),
                blank(&format!("{n}a"), &["kam"]),
                text("ich spät nach Hause."),
            ],
            hint: Some("kommen".to_string()),
        }
    }

    fn sheet() -> Sheet {
        Sheet {
            topic_id: "verbs_past.praeteritum_sein".to_string(),
            language: "de".to_string(),
            items: (1..=ITEMS_PER_SHEET as u32).map(item).collect(),
        }
    }

    #[test]
    fn a_good_sheet_validates() {
        sheet().validate().unwrap();
    }

    #[test]
    fn item_count_is_exact() {
        let mut s = sheet();
        s.items.pop();
        assert_eq!(s.validate(), Err(SheetError::WrongItemCount(19)));
    }

    #[test]
    fn every_item_needs_a_blank() {
        let mut s = sheet();
        s.items[4].segments = vec![text("Kein Platz zum Schreiben.")];
        assert_eq!(s.validate(), Err(SheetError::ItemWithoutBlank { item: 5 }));
    }

    #[test]
    fn every_blank_needs_an_answer() {
        let mut s = sheet();
        s.items[2].segments[1] = blank("3a", &[]);
        assert_eq!(
            s.validate(),
            Err(SheetError::EmptyAcceptList {
                blank: "3a".to_string()
            })
        );
    }

    #[test]
    fn the_answer_must_not_sit_in_the_sentence() {
        let mut s = sheet();
        s.items[0].segments = vec![
            text("Gestern kam ich, und"),
            blank("1a", &["kam"]),
            text("ich wieder."),
        ];
        assert_eq!(
            s.validate(),
            Err(SheetError::LeakedAnswer {
                item: 1,
                answer: "kam".to_string()
            })
        );
    }

    #[test]
    fn a_longer_word_containing_the_answer_is_not_a_leak() {
        let mut s = sheet();
        s.items[0].segments = vec![
            text("Wir mussten warten, also"),
            blank("1a", &["war"]),
            text("ich müde."),
        ];
        s.validate().unwrap();
    }

    #[test]
    fn blank_ids_are_unique_across_the_sheet() {
        let mut s = sheet();
        s.items[1].segments[1] = blank("1a", &["war"]);
        assert_eq!(
            s.validate(),
            Err(SheetError::DuplicateBlankId {
                blank: "1a".to_string()
            })
        );
    }

    #[test]
    fn item_numbers_must_be_in_order() {
        let mut s = sheet();
        s.items[3].n = 9;
        assert_eq!(
            s.validate(),
            Err(SheetError::BadItemNumber {
                found: 9,
                expected: 4
            })
        );
    }

    #[test]
    fn sentence_initial_is_only_the_leading_blank() {
        let item = Item {
            n: 1,
            segments: vec![
                text("  "),
                blank("1a", &["Gestern"]),
                text("kam ich und"),
                blank("1b", &["war"]),
                text("müde."),
            ],
            hint: None,
        };
        assert!(item.is_sentence_initial("1a"));
        assert!(!item.is_sentence_initial("1b"));
    }

    #[test]
    fn also_accept_extends_the_list_once() {
        let mut s = sheet();
        assert!(s.accept_also("1a", "ankam"));
        assert!(!s.accept_also("1a", "ankam"));
        assert_eq!(s.blank("1a").unwrap().accept, ["kam", "ankam"]);
        assert!(!s.accept_also("nope", "x"));
    }
}
