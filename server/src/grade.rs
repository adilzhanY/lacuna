//! Grading a filled in sheet.
//!
//! This is where the bugs will be, so every rule here has a test. German makes
//! three demands that a naive string compare gets wrong:
//!
//! 1. Nouns are capitalised, so `haus` is not `Haus`. But a blank at the start
//!    of a sentence is capitalised for position, not for grammar.
//! 2. `Straße` and `Strasse` are both real spellings, and so are `für` and
//!    `fuer` on a keyboard without umlauts.
//! 3. A trailing full stop the user typed is not a grammar mistake.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::{ErrorKind, ErrorTag, Rating};
use crate::sheet::{Blank, Item, Sheet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// Exactly right.
    Correct,
    /// Counted as right, with something worth knowing.
    CorrectWithNote { note: String, expected: String },
    /// Wrong. `expected` is the canonical answer, shown in place of a summary.
    Wrong {
        expected: String,
        tags: Vec<ErrorTag>,
    },
}

impl Verdict {
    pub fn is_correct(&self) -> bool {
        !matches!(self, Verdict::Wrong { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlankResult {
    pub blank_id: String,
    pub given: String,
    #[serde(flatten)]
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradedSheet {
    pub results: Vec<BlankResult>,
    pub correct: usize,
    pub total: usize,
    /// 0.0 to 1.0.
    pub score: f32,
    pub rating: Rating,
}

/// Grade a whole sheet. A blank with no answer counts as wrong.
pub fn grade_sheet(sheet: &Sheet, answers: &HashMap<String, String>) -> GradedSheet {
    let mut results = Vec::new();

    for item in &sheet.items {
        for blank in item.blanks() {
            let given = answers.get(&blank.id).cloned().unwrap_or_default();
            let verdict = grade_blank(blank, item, &given);
            results.push(BlankResult {
                blank_id: blank.id.clone(),
                given,
                verdict,
            });
        }
    }

    let total = results.len();
    let correct = results.iter().filter(|r| r.verdict.is_correct()).count();
    let score = if total == 0 {
        0.0
    } else {
        correct as f32 / total as f32
    };

    GradedSheet {
        results,
        correct,
        total,
        score,
        rating: rating_for(score),
    }
}

/// Grade one blank against everything its accept list allows.
pub fn grade_blank(blank: &Blank, item: &Item, given: &str) -> Verdict {
    let given = clean(given);
    let expected = blank.canonical().to_string();

    if given.is_empty() {
        return Verdict::Wrong {
            expected,
            tags: blank.tags.clone(),
        };
    }

    let sentence_initial = item.is_sentence_initial(&blank.id);

    // Pass one: exactly one of the accepted answers.
    if blank.accept.iter().any(|a| clean(a) == given) {
        return Verdict::Correct;
    }

    // Pass two: right letters, wrong capitals.
    for accepted in &blank.accept {
        let accepted = clean(accepted);
        if !eq_ignore_case(&accepted, &given) {
            continue;
        }
        // A word that is normally lowercase may be capitalised when it opens the
        // sentence. Nothing else is forgiven, because noun capitalisation is
        // part of German grammar, not decoration.
        if sentence_initial && starts_lowercase(&accepted) && starts_uppercase(&given) {
            return Verdict::Correct;
        }
        return Verdict::Wrong {
            expected: accepted,
            tags: vec![ErrorTag::new(ErrorKind::Capitalisation, "letter_case")],
        };
    }

    // Pass three: umlauts and sharp s written the long way, `fuer` for `für`.
    for accepted in &blank.accept {
        let accepted = clean(accepted);
        if fold(&accepted) != fold(&given) {
            continue;
        }
        // The letters are equivalent. Capitals are judged on the transliterated
        // forms, so "Strasse" is fine for "Straße" but "STRASSE" is not.
        let transliterated = transliterate(&accepted);
        let capitals_ok = transliterated == given
            || (sentence_initial
                && starts_lowercase(&transliterated)
                && starts_uppercase(&given)
                && eq_ignore_case(&transliterated, &given));
        if !capitals_ok {
            return Verdict::Wrong {
                expected: accepted,
                tags: vec![ErrorTag::new(ErrorKind::Capitalisation, "letter_case")],
            };
        }
        return Verdict::CorrectWithNote {
            note: format!("`{accepted}` is the normal spelling"),
            expected: accepted,
        };
    }

    Verdict::Wrong {
        expected,
        tags: blank.tags.clone(),
    }
}

/// Score to FSRS button. The boundaries are deliberate: a sheet where one blank
/// in five is wrong is not a sheet you have learned.
pub fn rating_for(score: f32) -> Rating {
    if score < 0.60 {
        Rating::Again
    } else if score < 0.80 {
        Rating::Hard
    } else if score < 0.95 {
        Rating::Good
    } else {
        Rating::Easy
    }
}

/// Trim, collapse inner whitespace, and drop punctuation the user typed around
/// the word. Hyphens and apostrophes stay, they carry meaning.
fn clean(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .trim_matches(|c: char| c.is_ascii_punctuation() && c != '-' && c != '\'')
        .to_string()
}

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn starts_uppercase(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_uppercase)
}

fn starts_lowercase(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_lowercase)
}

/// Rewrite umlauts and sharp s the long way, keeping the capitals as they are.
/// `Straße` becomes `Strasse`, `Ärger` becomes `Aerger`.
fn transliterate(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            'Ä' => out.push_str("Ae"),
            'Ö' => out.push_str("Oe"),
            'Ü' => out.push_str("Ue"),
            other => out.push(other),
        }
    }
    out
}

/// The comparison form: transliterated and lowercase, so `für`, `fuer` and
/// `FUER` all collapse to the same string.
fn fold(s: &str) -> String {
    transliterate(s).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::{Blank, Segment};

    fn item_with(accept: &[&str], leading_blank: bool) -> (Item, Blank) {
        let blank = Blank {
            id: "1a".to_string(),
            accept: accept.iter().map(|s| s.to_string()).collect(),
            tags: vec![ErrorTag::new(ErrorKind::Case, "dative")],
        };
        let segments = if leading_blank {
            vec![
                Segment::Blank(blank.clone()),
                Segment::Text {
                    text: "ist mein Haus.".to_string(),
                },
            ]
        } else {
            vec![
                Segment::Text {
                    text: "Das ist".to_string(),
                },
                Segment::Blank(blank.clone()),
                Segment::Text {
                    text: "Haus.".to_string(),
                },
            ]
        };
        (
            Item {
                n: 1,
                segments,
                hint: None,
            },
            blank,
        )
    }

    fn grade(accept: &[&str], given: &str) -> Verdict {
        let (item, blank) = item_with(accept, false);
        grade_blank(&blank, &item, given)
    }

    #[test]
    fn exact_answer_is_correct() {
        assert_eq!(grade(&["dem"], "dem"), Verdict::Correct);
    }

    #[test]
    fn any_accepted_answer_counts() {
        assert_eq!(grade(&["dem", "diesem"], "diesem"), Verdict::Correct);
    }

    #[test]
    fn whitespace_and_stray_punctuation_are_forgiven() {
        assert_eq!(grade(&["dem"], "  dem "), Verdict::Correct);
        assert_eq!(grade(&["dem"], "dem."), Verdict::Correct);
        assert_eq!(grade(&["dem"], "\"dem\""), Verdict::Correct);
    }

    #[test]
    fn an_empty_answer_is_wrong() {
        let verdict = grade(&["dem"], "   ");
        assert!(matches!(verdict, Verdict::Wrong { .. }));
    }

    #[test]
    fn a_wrong_answer_carries_the_blanks_tags() {
        let Verdict::Wrong { expected, tags } = grade(&["dem"], "den") else {
            panic!("expected a wrong verdict");
        };
        assert_eq!(expected, "dem");
        assert_eq!(tags, vec![ErrorTag::new(ErrorKind::Case, "dative")]);
    }

    #[test]
    fn a_lowercase_noun_is_wrong() {
        let Verdict::Wrong { expected, tags } = grade(&["Haus"], "haus") else {
            panic!("noun capitalisation must count");
        };
        assert_eq!(expected, "Haus");
        assert_eq!(tags[0].kind, ErrorKind::Capitalisation);
    }

    #[test]
    fn a_capital_in_the_middle_of_a_sentence_is_wrong() {
        let verdict = grade(&["dem"], "Dem");
        assert!(matches!(verdict, Verdict::Wrong { .. }));
    }

    #[test]
    fn a_capital_at_the_start_of_a_sentence_is_fine() {
        let (item, blank) = item_with(&["dieses"], true);
        assert_eq!(grade_blank(&blank, &item, "Dieses"), Verdict::Correct);
    }

    #[test]
    fn ss_for_sharp_s_is_accepted_with_a_note() {
        let Verdict::CorrectWithNote { expected, .. } = grade(&["Straße"], "Strasse") else {
            panic!("ss should be accepted");
        };
        assert_eq!(expected, "Straße");
    }

    #[test]
    fn ae_for_umlaut_is_accepted_with_a_note() {
        assert!(matches!(
            grade(&["für"], "fuer"),
            Verdict::CorrectWithNote { .. }
        ));
        assert!(matches!(
            grade(&["Bäcker"], "Baecker"),
            Verdict::CorrectWithNote { .. }
        ));
    }

    #[test]
    fn shouting_the_answer_is_still_a_capitalisation_mistake() {
        let Verdict::Wrong { tags, .. } = grade(&["Straße"], "STRASSE") else {
            panic!("expected a capitalisation mistake");
        };
        assert_eq!(tags[0].kind, ErrorKind::Capitalisation);
    }

    #[test]
    fn a_real_typo_is_still_wrong() {
        assert!(matches!(grade(&["für"], "fur"), Verdict::Wrong { .. }));
    }

    #[test]
    fn score_boundaries() {
        assert_eq!(rating_for(0.0), Rating::Again);
        assert_eq!(rating_for(0.59), Rating::Again);
        assert_eq!(rating_for(0.60), Rating::Hard);
        assert_eq!(rating_for(0.79), Rating::Hard);
        assert_eq!(rating_for(0.80), Rating::Good);
        assert_eq!(rating_for(0.94), Rating::Good);
        assert_eq!(rating_for(0.95), Rating::Easy);
        assert_eq!(rating_for(1.0), Rating::Easy);
    }

    #[test]
    fn grading_a_sheet_counts_missing_answers_as_wrong() {
        let sheet = Sheet {
            topic_id: "cases.dative".to_string(),
            language: "de".to_string(),
            items: vec![Item {
                n: 1,
                segments: vec![
                    Segment::Text {
                        text: "Ich fahre mit".to_string(),
                    },
                    Segment::Blank(Blank {
                        id: "1a".to_string(),
                        accept: vec!["dem".to_string()],
                        tags: vec![],
                    }),
                    Segment::Text {
                        text: "Bus zur".to_string(),
                    },
                    Segment::Blank(Blank {
                        id: "1b".to_string(),
                        accept: vec!["Arbeit".to_string()],
                        tags: vec![],
                    }),
                ],
                hint: None,
            }],
        };

        let mut answers = HashMap::new();
        answers.insert("1a".to_string(), "dem".to_string());
        let graded = grade_sheet(&sheet, &answers);

        assert_eq!(graded.total, 2);
        assert_eq!(graded.correct, 1);
        assert_eq!(graded.rating, Rating::Again);
    }
}
