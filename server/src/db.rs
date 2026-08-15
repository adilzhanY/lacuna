//! SQLite access.
//!
//! Queries are written at runtime rather than through `sqlx::query!`, so the
//! crate builds without a database present. The row shapes are small and every
//! one of them is covered by a test against a real in memory database.

use std::str::FromStr;

use chrono::NaiveDate;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::domain::Rating;
use crate::grade::GradedSheet;
use crate::pack::Pack;
use crate::schedule::TopicMemory;
use crate::sheet::Sheet;

/// Open (and create if missing) the database, then run the migrations.
pub async fn connect(url: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Copy the pack's topics into the database, keeping any scheduling state that
/// already exists. The pack file stays the source of truth for the tree.
pub async fn sync_pack(pool: &SqlitePool, pack: &Pack) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    for topic in &pack.topics {
        sqlx::query(
            "insert into topic (id, cefr, stage, category, title, goal, status)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             on conflict (id) do update set
                 cefr = excluded.cefr,
                 stage = excluded.stage,
                 category = excluded.category,
                 title = excluded.title,
                 goal = excluded.goal,
                 status = excluded.status",
        )
        .bind(&topic.id)
        .bind(topic.cefr.as_str())
        .bind(topic.stage)
        .bind(&topic.category)
        .bind(&topic.title)
        .bind(&topic.goal)
        .bind(serde_json::to_value(topic.status)?.as_str().unwrap_or("known"))
        .execute(&mut *tx)
        .await?;

        sqlx::query("insert or ignore into topic_state (topic_id) values (?1)")
            .bind(&topic.id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopicState {
    pub topic_id: String,
    pub memory: Option<TopicMemory>,
    pub due: Option<NaiveDate>,
    pub last_review: Option<NaiveDate>,
    pub reps: i64,
    pub lapses: i64,
}

impl TopicState {
    /// A topic is due when it has never been studied, or its due date has come.
    pub fn is_due(&self, today: NaiveDate) -> bool {
        match self.due {
            None => true,
            Some(due) => due <= today,
        }
    }

    pub fn days_since_review(&self, today: NaiveDate) -> u32 {
        match self.last_review {
            None => 0,
            Some(last) => (today - last).num_days().max(0) as u32,
        }
    }
}

fn state_from_row(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<TopicState> {
    let stability: Option<f64> = row.try_get("stability")?;
    let difficulty: Option<f64> = row.try_get("difficulty")?;
    let memory = match (stability, difficulty) {
        (Some(s), Some(d)) => Some(TopicMemory {
            stability: s as f32,
            difficulty: d as f32,
        }),
        _ => None,
    };
    let due: Option<String> = row.try_get("due")?;
    let last_review: Option<String> = row.try_get("last_review")?;
    Ok(TopicState {
        topic_id: row.try_get("topic_id")?,
        memory,
        due: due.map(|d| d.parse()).transpose()?,
        last_review: last_review.map(|d| d.parse()).transpose()?,
        reps: row.try_get("reps")?,
        lapses: row.try_get("lapses")?,
    })
}

pub async fn topic_states(pool: &SqlitePool) -> anyhow::Result<Vec<TopicState>> {
    let rows = sqlx::query("select * from topic_state").fetch_all(pool).await?;
    rows.iter().map(state_from_row).collect()
}

pub async fn topic_state(pool: &SqlitePool, topic_id: &str) -> anyhow::Result<Option<TopicState>> {
    let row = sqlx::query("select * from topic_state where topic_id = ?1")
        .bind(topic_id)
        .fetch_optional(pool)
        .await?;
    row.as_ref().map(state_from_row).transpose()
}

pub async fn save_sheet(pool: &SqlitePool, sheet: &Sheet, created: NaiveDate) -> anyhow::Result<i64> {
    let body = serde_json::to_string(sheet)?;
    let row = sqlx::query(
        "insert into sheet (topic_id, language, body, created_at)
         values (?1, ?2, ?3, ?4) returning id",
    )
    .bind(&sheet.topic_id)
    .bind(&sheet.language)
    .bind(body)
    .bind(created.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.try_get("id")?)
}

pub async fn load_sheet(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<Sheet>> {
    let row = sqlx::query("select body from sheet where id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let body: String = row.try_get("body")?;
            Ok(Some(serde_json::from_str(&body)?))
        }
    }
}

/// The newest sheet stored for a topic, if there is one.
pub async fn latest_sheet_for(pool: &SqlitePool, topic_id: &str) -> anyhow::Result<Option<(i64, Sheet)>> {
    let row = sqlx::query("select id, body from sheet where topic_id = ?1 order by id desc limit 1")
        .bind(topic_id)
        .fetch_optional(pool)
        .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let id: i64 = row.try_get("id")?;
            let body: String = row.try_get("body")?;
            Ok(Some((id, serde_json::from_str(&body)?)))
        }
    }
}

/// Overwrite a stored sheet, which is what "also accept" does.
pub async fn update_sheet(pool: &SqlitePool, id: i64, sheet: &Sheet) -> anyhow::Result<()> {
    let body = serde_json::to_string(sheet)?;
    sqlx::query("update sheet set body = ?1 where id = ?2")
        .bind(body)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Write the review and every answer in it, then move the topic's schedule on.
#[allow(clippy::too_many_arguments)]
pub async fn record_review(
    pool: &SqlitePool,
    sheet_id: i64,
    topic_id: &str,
    graded: &GradedSheet,
    rating: Rating,
    memory: TopicMemory,
    due: NaiveDate,
    today: NaiveDate,
) -> anyhow::Result<i64> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        "insert into review (sheet_id, topic_id, reviewed_at, correct, total, score, rating)
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7) returning id",
    )
    .bind(sheet_id)
    .bind(topic_id)
    .bind(today.to_string())
    .bind(graded.correct as i64)
    .bind(graded.total as i64)
    .bind(graded.score as f64)
    .bind(rating.as_str())
    .fetch_one(&mut *tx)
    .await?;
    let review_id: i64 = row.try_get("id")?;

    for result in &graded.results {
        let (expected, tags) = match &result.verdict {
            crate::grade::Verdict::Correct => (String::new(), String::new()),
            crate::grade::Verdict::CorrectWithNote { expected, .. } => (expected.clone(), String::new()),
            crate::grade::Verdict::Wrong { expected, tags } => (
                expected.clone(),
                tags.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" "),
            ),
        };
        sqlx::query(
            "insert into answer (review_id, blank_id, given, expected, correct, tags)
             values (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(review_id)
        .bind(&result.blank_id)
        .bind(&result.given)
        .bind(expected)
        .bind(i64::from(result.verdict.is_correct()))
        .bind(tags)
        .execute(&mut *tx)
        .await?;
    }

    let lapse = i64::from(rating == Rating::Again);
    sqlx::query(
        "update topic_state
            set stability = ?1,
                difficulty = ?2,
                due = ?3,
                last_review = ?4,
                reps = reps + 1,
                lapses = lapses + ?5
          where topic_id = ?6",
    )
    .bind(memory.stability as f64)
    .bind(memory.difficulty as f64)
    .bind(due.to_string())
    .bind(today.to_string())
    .bind(lapse)
    .bind(topic_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(review_id)
}

/// How often each error tag has come up, worst first. This is the data that
/// makes "your dative only fails after two way prepositions" possible.
pub async fn error_tag_counts(pool: &SqlitePool) -> anyhow::Result<Vec<(String, i64)>> {
    let rows = sqlx::query("select tags from answer where correct = 0 and tags != ''")
        .fetch_all(pool)
        .await?;
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for row in rows {
        let tags: String = row.try_get("tags")?;
        for tag in tags.split_whitespace() {
            *counts.entry(tag.to_string()).or_default() += 1;
        }
    }
    let mut out: Vec<(String, i64)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grade::grade_sheet;
    use crate::sheet::{Blank, Item, Segment};
    use std::collections::HashMap;
    use std::path::Path;

    async fn setup() -> (SqlitePool, Pack) {
        let pool = connect("sqlite::memory:").await.unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs");
        let pack = Pack::load(&root, "de").unwrap();
        sync_pack(&pool, &pack).await.unwrap();
        (pool, pack)
    }

    fn tiny_sheet() -> Sheet {
        Sheet {
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
                        tags: vec!["case:dative".parse().unwrap()],
                    }),
                    Segment::Text {
                        text: "Bus.".to_string(),
                    },
                ],
                hint: None,
            }],
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
    }

    #[tokio::test]
    async fn syncing_the_pack_creates_topics_and_states() {
        let (pool, pack) = setup().await;
        let states = topic_states(&pool).await.unwrap();
        assert_eq!(states.len(), pack.topics.len());
        assert!(states.iter().all(|s| s.memory.is_none()));
        assert!(states.iter().all(|s| s.is_due(today())));
    }

    #[tokio::test]
    async fn syncing_twice_is_harmless() {
        let (pool, pack) = setup().await;
        sync_pack(&pool, &pack).await.unwrap();
        assert_eq!(topic_states(&pool).await.unwrap().len(), pack.topics.len());
    }

    #[tokio::test]
    async fn sheets_round_trip() {
        let (pool, _) = setup().await;
        let sheet = tiny_sheet();
        let id = save_sheet(&pool, &sheet, today()).await.unwrap();
        assert_eq!(load_sheet(&pool, id).await.unwrap().unwrap(), sheet);
        let (latest_id, latest) = latest_sheet_for(&pool, "cases.dative").await.unwrap().unwrap();
        assert_eq!(latest_id, id);
        assert_eq!(latest, sheet);
    }

    #[tokio::test]
    async fn also_accept_survives_a_reload() {
        let (pool, _) = setup().await;
        let mut sheet = tiny_sheet();
        let id = save_sheet(&pool, &sheet, today()).await.unwrap();
        assert!(sheet.accept_also("1a", "diesem"));
        update_sheet(&pool, id, &sheet).await.unwrap();
        let reloaded = load_sheet(&pool, id).await.unwrap().unwrap();
        assert_eq!(reloaded.blank("1a").unwrap().accept, ["dem", "diesem"]);
    }

    #[tokio::test]
    async fn recording_a_review_moves_the_schedule_and_logs_answers() {
        let (pool, _) = setup().await;
        let sheet = tiny_sheet();
        let sheet_id = save_sheet(&pool, &sheet, today()).await.unwrap();

        let mut answers = HashMap::new();
        answers.insert("1a".to_string(), "den".to_string());
        let graded = grade_sheet(&sheet, &answers);

        let memory = TopicMemory {
            stability: 3.0,
            difficulty: 5.0,
        };
        let due = NaiveDate::from_ymd_opt(2026, 8, 18).unwrap();
        record_review(
            &pool,
            sheet_id,
            "cases.dative",
            &graded,
            graded.rating,
            memory,
            due,
            today(),
        )
        .await
        .unwrap();

        let state = topic_state(&pool, "cases.dative").await.unwrap().unwrap();
        assert_eq!(state.memory, Some(memory));
        assert_eq!(state.due, Some(due));
        assert_eq!(state.last_review, Some(today()));
        assert_eq!(state.reps, 1);
        assert_eq!(state.lapses, 1, "a failed sheet is a lapse");
        assert!(!state.is_due(today()));

        let counts = error_tag_counts(&pool).await.unwrap();
        assert_eq!(counts, vec![("case:dative".to_string(), 1)]);
    }

    #[tokio::test]
    async fn days_since_review_counts_from_the_last_one() {
        let state = TopicState {
            topic_id: "x".to_string(),
            memory: None,
            due: None,
            last_review: Some(NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()),
            reps: 1,
            lapses: 0,
        };
        assert_eq!(state.days_since_review(today()), 5);
    }
}
