//! Review mode: one sentence at a time, graded on correctness and speed.
//!
//! The clock is never shown to the reader. It exists so that an answer typed
//! straight away and an answer typed after five seconds of thinking do not earn
//! the same rating. Recall that has to be reconstructed is weaker recall, which
//! is exactly what the scheduler wants to know.
//!
//! The budget for an item is worked out from the item itself rather than a flat
//! number, because "Ich fahre mit ___ Bus." and a twenty word sentence with two
//! blanks are not the same amount of work.

use serde::{Deserialize, Serialize};

use crate::domain::Rating;
use crate::grade::rating_for;
use crate::sheet::Item;

/// Fixed cost of taking in a new sentence.
const READ_BASE_MS: f32 = 400.0;
/// Reading speed, about 200 words a minute for a non native reader.
const READ_PER_CHAR_MS: f32 = 55.0;
/// Typing speed, deliberately generous: German on a keyboard you may be fighting.
const TYPE_PER_CHAR_MS: f32 = 260.0;

/// Answered inside the budget.
const EASY_RATIO: f32 = 1.0;
/// Answered inside a bit under twice the budget.
const GOOD_RATIO: f32 = 1.8;

/// How long this item should reasonably take, in milliseconds.
pub fn expected_ms(item: &Item) -> u32 {
    let visible = item.visible_text().chars().count() as f32;
    let typing: f32 = item
        .blanks()
        .map(|b| b.canonical().chars().count() as f32 * TYPE_PER_CHAR_MS)
        .sum();
    (READ_BASE_MS + visible * READ_PER_CHAR_MS + typing).round() as u32
}

/// The rating one item earns.
///
/// A mistake is always `Again`, whatever the clock says. Speed only separates
/// the answers that were right.
pub fn grade_item(item: &Item, elapsed_ms: u32, all_correct: bool) -> Rating {
    if !all_correct {
        return Rating::Again;
    }
    let budget = expected_ms(item).max(1) as f32;
    let ratio = elapsed_ms as f32 / budget;
    if ratio <= EASY_RATIO {
        Rating::Easy
    } else if ratio <= GOOD_RATIO {
        Rating::Good
    } else {
        Rating::Hard
    }
}

/// What one item rating is worth when the whole session is averaged.
fn weight(rating: Rating) -> f32 {
    match rating {
        Rating::Again => 0.0,
        Rating::Hard => 0.6,
        Rating::Good => 0.85,
        Rating::Easy => 1.0,
    }
}

/// The rating the topic gets from a finished run.
///
/// The mean is mapped through the same thresholds a checked sheet uses, then one
/// rule is applied on top: a perfect rating needs a perfect run. Nineteen instant
/// answers and one mistake is a good session, not an easy one.
pub fn session_rating(item_ratings: &[Rating]) -> Rating {
    if item_ratings.is_empty() {
        return Rating::Again;
    }
    let mean: f32 =
        item_ratings.iter().copied().map(weight).sum::<f32>() / item_ratings.len() as f32;
    let rating = rating_for(mean);
    let mistakes = item_ratings.iter().filter(|r| **r == Rating::Again).count();
    if mistakes > 0 && rating == Rating::Easy {
        Rating::Good
    } else {
        rating
    }
}

/// One item as the browser reports it: what was typed, and how long it took from
/// the sentence appearing to the answer being sent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemAttempt {
    pub n: u32,
    pub elapsed_ms: u32,
    /// Blank id to what was typed.
    pub answers: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::{Blank, Segment};

    fn item(text: &str, answer: &str) -> Item {
        Item {
            n: 1,
            segments: vec![
                Segment::Text {
                    text: text.to_string(),
                },
                Segment::Blank(Blank {
                    id: "1a".to_string(),
                    accept: vec![answer.to_string()],
                    tags: vec![],
                }),
                Segment::Text {
                    text: "Bus.".to_string(),
                },
            ],
            hint: None,
        }
    }

    #[test]
    fn a_longer_sentence_gets_a_longer_budget() {
        let short = item("Ich fahre mit", "dem");
        let long = item(
            "Gestern Abend bin ich noch schnell zum Supermarkt gelaufen und dann mit",
            "dem",
        );
        assert!(expected_ms(&long) > expected_ms(&short));
    }

    #[test]
    fn a_longer_answer_gets_a_longer_budget() {
        let short = item("Ich fahre mit", "dem");
        let long = item("Ich fahre mit", "demjenigen");
        assert!(expected_ms(&long) > expected_ms(&short));
    }

    #[test]
    fn a_mistake_is_always_again_however_fast() {
        let item = item("Ich fahre mit", "dem");
        assert_eq!(grade_item(&item, 1, false), Rating::Again);
        assert_eq!(grade_item(&item, 60_000, false), Rating::Again);
    }

    #[test]
    fn instant_and_correct_is_easy() {
        let item = item("Ich fahre mit", "dem");
        assert_eq!(grade_item(&item, 300, true), Rating::Easy);
    }

    #[test]
    fn correct_but_slow_is_good_not_easy() {
        // This is the case from the brief: right answer, five seconds of thinking.
        let item = item("Ich fahre mit", "dem");
        let budget = expected_ms(&item);
        assert!(budget < 5_000, "a short item should not budget five seconds");
        assert_eq!(grade_item(&item, 5_000, true), Rating::Hard);
        assert_eq!(grade_item(&item, budget + 100, true), Rating::Good);
    }

    #[test]
    fn the_boundaries_are_where_they_say_they_are() {
        let item = item("Ich fahre mit", "dem");
        let budget = expected_ms(&item);
        assert_eq!(grade_item(&item, budget, true), Rating::Easy);
        assert_eq!(grade_item(&item, budget + 1, true), Rating::Good);
        let good_edge = (budget as f32 * GOOD_RATIO) as u32;
        assert_eq!(grade_item(&item, good_edge, true), Rating::Good);
        assert_eq!(grade_item(&item, good_edge + 500, true), Rating::Hard);
    }

    #[test]
    fn a_perfect_fast_run_is_easy() {
        assert_eq!(session_rating(&[Rating::Easy; 20]), Rating::Easy);
    }

    #[test]
    fn one_mistake_caps_the_session_at_good() {
        let mut ratings = vec![Rating::Easy; 19];
        ratings.push(Rating::Again);
        assert_eq!(session_rating(&ratings), Rating::Good);
    }

    #[test]
    fn a_slow_but_correct_run_is_good() {
        assert_eq!(session_rating(&[Rating::Good; 20]), Rating::Good);
    }

    #[test]
    fn a_laboured_run_is_hard() {
        assert_eq!(session_rating(&[Rating::Hard; 20]), Rating::Hard);
    }

    #[test]
    fn a_run_full_of_mistakes_is_again() {
        let mut ratings = vec![Rating::Again; 10];
        ratings.extend([Rating::Easy; 10]);
        assert_eq!(session_rating(&ratings), Rating::Again);
    }

    #[test]
    fn an_empty_run_is_again() {
        assert_eq!(session_rating(&[]), Rating::Again);
    }
}
