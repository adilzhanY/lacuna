//! Topic scheduling.
//!
//! The algorithm is FSRS, the same one Anki uses, through the first party
//! `fsrs` crate. lacuna schedules topics, not cards, so the memory state hangs
//! off a topic id and the "review" that drives it is a whole sheet.

use chrono::NaiveDate;
use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};

use crate::domain::Rating;

/// How much of a topic you want to still remember when it comes back.
pub const DEFAULT_RETENTION: f32 = 0.9;

/// The stored memory of one topic.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TopicMemory {
    pub stability: f32,
    pub difficulty: f32,
}

impl From<MemoryState> for TopicMemory {
    fn from(m: MemoryState) -> Self {
        Self {
            stability: m.stability,
            difficulty: m.difficulty,
        }
    }
}

impl From<TopicMemory> for MemoryState {
    fn from(m: TopicMemory) -> Self {
        MemoryState {
            stability: m.stability,
            difficulty: m.difficulty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scheduled {
    pub memory: TopicMemory,
    /// Whole days until the topic is due again, at least 1.
    pub interval_days: i64,
    pub due: NaiveDate,
}

pub struct Scheduler {
    fsrs: FSRS,
    retention: f32,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION)
    }
}

impl Scheduler {
    pub fn new(retention: f32) -> Self {
        Self {
            // An empty parameter slice means the default FSRS parameters, which
            // is what we want until there is enough review history to optimise.
            fsrs: FSRS::new(&[]).expect("default FSRS parameters are valid"),
            retention: retention.clamp(0.70, 0.99),
        }
    }

    /// Apply one review to a topic.
    ///
    /// `current` is `None` the first time a topic is ever studied.
    /// `days_elapsed` is how long it has been since the previous review.
    pub fn review(
        &self,
        current: Option<TopicMemory>,
        days_elapsed: u32,
        rating: Rating,
        today: NaiveDate,
    ) -> anyhow::Result<Scheduled> {
        let states = self
            .fsrs
            .next_states(current.map(Into::into), self.retention, days_elapsed)
            .map_err(|e| anyhow::anyhow!("fsrs could not schedule: {e}"))?;

        let next = match rating {
            Rating::Again => states.again,
            Rating::Hard => states.hard,
            Rating::Good => states.good,
            Rating::Easy => states.easy,
        };

        let interval_days = (next.interval.round() as i64).max(1);
        let due = today
            .checked_add_signed(chrono::Duration::days(interval_days))
            .ok_or_else(|| anyhow::anyhow!("due date overflowed"))?;

        Ok(Scheduled {
            memory: next.memory.into(),
            interval_days,
            due,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
    }

    #[test]
    fn a_new_topic_gets_a_memory_state() {
        let s = Scheduler::default();
        let out = s.review(None, 0, Rating::Good, today()).unwrap();
        assert!(out.memory.stability > 0.0);
        assert!(out.interval_days >= 1);
        assert!(out.due > today());
    }

    #[test]
    fn better_ratings_push_the_topic_further_out() {
        let s = Scheduler::default();
        let again = s.review(None, 0, Rating::Again, today()).unwrap();
        let hard = s.review(None, 0, Rating::Hard, today()).unwrap();
        let good = s.review(None, 0, Rating::Good, today()).unwrap();
        let easy = s.review(None, 0, Rating::Easy, today()).unwrap();

        assert!(again.interval_days <= hard.interval_days);
        assert!(hard.interval_days <= good.interval_days);
        assert!(good.interval_days <= easy.interval_days);
    }

    #[test]
    fn repeated_good_reviews_grow_the_interval() {
        let s = Scheduler::default();
        let first = s.review(None, 0, Rating::Good, today()).unwrap();
        let second = s
            .review(
                Some(first.memory),
                first.interval_days as u32,
                Rating::Good,
                first.due,
            )
            .unwrap();
        assert!(second.interval_days > first.interval_days);
    }

    #[test]
    fn again_after_a_long_streak_shortens_the_interval() {
        let s = Scheduler::default();
        let grown = s.review(None, 0, Rating::Easy, today()).unwrap();
        let lapsed = s
            .review(
                Some(grown.memory),
                grown.interval_days as u32,
                Rating::Again,
                grown.due,
            )
            .unwrap();
        assert!(lapsed.interval_days < grown.interval_days);
    }

    #[test]
    fn retention_is_clamped_to_something_sane() {
        let s = Scheduler::new(2.0);
        assert!(s.retention <= 0.99);
        let s = Scheduler::new(0.1);
        assert!(s.retention >= 0.70);
    }
}
