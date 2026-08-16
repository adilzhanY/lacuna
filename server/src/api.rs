//! HTTP surface.
//!
//! The client never receives an accept list. A sheet leaves this module with
//! its blanks reduced to ids, so the answers only exist on the server until the
//! sheet is checked.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Datelike, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db;
use crate::domain::{Level, Rating};
use crate::grade::{grade_blank, grade_sheet, rating_for, BlankResult, GradedSheet, Verdict};
use crate::review::{grade_item, session_rating};
use crate::sheet::{Segment, Sheet};
use crate::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/topics", get(topics))
        .route("/api/today", get(today_queue))
        .route("/api/weaknesses", get(weaknesses))
        .route("/api/stats", get(stats))
        .route("/api/sheet/{topic_id}", get(sheet_for_topic))
        .route("/api/sheet/{sheet_id}/check", post(check_sheet))
        .route("/api/sheet/{sheet_id}/accept", post(accept_also))
        .route("/api/review/next", get(review_next))
        .route("/api/review/{sheet_id}/item", post(review_item))
        .route("/api/review/{sheet_id}/finish", post(review_finish))
        .with_state(state)
}

// ---------------------------------------------------------------- errors

pub struct ApiError(anyhow::Error, StatusCode);

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        ApiError(anyhow::anyhow!(message.into()), StatusCode::NOT_FOUND)
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        ApiError(err.into(), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let message = self.0.to_string();
        if self.1 == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("request failed: {message:#}");
        }
        (self.1, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

// ---------------------------------------------------------------- topics

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct TopicView {
    pub id: String,
    pub cefr: Level,
    pub stage: u32,
    pub category: String,
    pub title: String,
    pub goal: String,
    /// `null` until the topic has been studied once.
    pub due: Option<String>,
    #[ts(type = "number")]
    pub reps: i64,
    #[ts(type = "number")]
    pub lapses: i64,
    pub is_due: bool,
    pub is_new: bool,
    /// True when a sheet for this topic already exists on disk or in the database.
    pub has_sheet: bool,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn topics(State(state): State<SharedState>) -> ApiResult<Vec<TopicView>> {
    Ok(Json(topic_views(&state).await?))
}

/// Topics that are due today, in teaching order.
async fn today_queue(State(state): State<SharedState>) -> ApiResult<Vec<TopicView>> {
    let mut views = topic_views(&state).await?;
    views.retain(|v| v.is_due && v.has_sheet);
    Ok(Json(views))
}

async fn topic_views(state: &SharedState) -> anyhow::Result<Vec<TopicView>> {
    let states = db::topic_states(&state.db).await?;
    let by_id: HashMap<&str, &db::TopicState> =
        states.iter().map(|s| (s.topic_id.as_str(), s)).collect();
    let today = today();

    let mut out = Vec::with_capacity(state.pack.topics.len());
    for topic in &state.pack.topics {
        let s = by_id.get(topic.id.as_str());
        let has_sheet = seed_path(state, &topic.id).exists()
            || db::latest_sheet_for(&state.db, &topic.id).await?.is_some();
        out.push(TopicView {
            id: topic.id.clone(),
            cefr: topic.cefr,
            stage: topic.stage,
            category: topic.category.clone(),
            title: topic.title.clone(),
            goal: topic.goal.clone(),
            due: s.and_then(|s| s.due).map(|d| d.to_string()),
            reps: s.map(|s| s.reps).unwrap_or(0),
            lapses: s.map(|s| s.lapses).unwrap_or(0),
            is_due: s.map(|s| s.is_due(today)).unwrap_or(true),
            is_new: s.map(|s| s.reps == 0).unwrap_or(true),
            has_sheet,
        });
    }
    Ok(out)
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct Weakness {
    pub tag: String,
    #[ts(type = "number")]
    pub count: i64,
}

async fn weaknesses(State(state): State<SharedState>) -> ApiResult<Vec<Weakness>> {
    let counts = db::error_tag_counts(&state.db).await?;
    Ok(Json(
        counts
            .into_iter()
            .map(|(tag, count)| Weakness { tag, count })
            .collect(),
    ))
}

// ---------------------------------------------------------------- statistics

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct LevelProgress {
    pub level: Level,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub studied: i64,
    #[ts(type = "number")]
    pub due: i64,
}

/// One column of a chart: a date and what happened on it.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct DayPoint {
    pub date: String,
    #[ts(type = "number")]
    pub count: i64,
    /// Share of blanks correct that day, absent when nothing was studied.
    pub accuracy: Option<f32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct TopicScore {
    pub id: String,
    pub title: String,
    pub cefr: Level,
    pub accuracy: f32,
    #[ts(type = "number")]
    pub reviews: i64,
    #[ts(type = "number")]
    pub lapses: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct Stats {
    pub today: String,
    #[ts(type = "number")]
    pub topics_total: i64,
    #[ts(type = "number")]
    pub topics_studied: i64,
    #[ts(type = "number")]
    pub topics_due: i64,
    #[ts(type = "number")]
    pub reviews_total: i64,
    #[ts(type = "number")]
    pub blanks_total: i64,
    #[ts(type = "number")]
    pub blanks_correct: i64,
    /// 0.0 to 1.0 across every blank ever answered.
    pub accuracy: f32,
    #[ts(type = "number")]
    pub streak_days: i64,
    pub by_level: Vec<LevelProgress>,
    /// One entry per day for the last 30 days, including empty ones.
    pub activity: Vec<DayPoint>,
    /// One entry per day for the next 14 days, including empty ones.
    pub forecast: Vec<DayPoint>,
    pub weakest: Vec<Weakness>,
    pub hardest: Vec<TopicScore>,
    /// One entry per day for the last year, starting on a Monday so the grid is square.
    pub year: Vec<DayPoint>,
    #[ts(type = "number")]
    pub longest_streak: i64,
    /// Share of days in the year window with at least one review.
    pub days_learned: f32,
    #[ts(type = "number")]
    pub today_reviews: i64,
    #[ts(type = "number")]
    pub today_blanks: i64,
    #[ts(type = "number")]
    pub today_ms: i64,
}

const ACTIVITY_DAYS: i64 = 30;
const YEAR_DAYS: i64 = 364;
const FORECAST_DAYS: i64 = 14;
const TOP_N: usize = 8;

async fn stats(State(state): State<SharedState>) -> ApiResult<Stats> {
    let today = today();
    let states = db::topic_states(&state.db).await?;
    let daily = db::daily_activity(&state.db).await?;
    let records = db::topic_records(&state.db).await?;
    let forecast_rows = db::due_forecast(&state.db, today).await?;
    let tags = db::error_tag_counts(&state.db).await?;

    let studied: HashMap<&str, &db::TopicRecord> =
        records.iter().map(|r| (r.topic_id.as_str(), r)).collect();

    // Levels, counted from the pack so empty levels still show as empty.
    let mut by_level = Vec::new();
    for level in Level::ALL {
        let in_level: Vec<&crate::pack::Topic> = state
            .pack
            .topics
            .iter()
            .filter(|t| t.cefr == level)
            .collect();
        if in_level.is_empty() {
            continue;
        }
        let due = states
            .iter()
            .filter(|s| s.is_due(today) && in_level.iter().any(|t| t.id == s.topic_id))
            .count() as i64;
        by_level.push(LevelProgress {
            level,
            total: in_level.len() as i64,
            studied: in_level
                .iter()
                .filter(|t| studied.contains_key(t.id.as_str()))
                .count() as i64,
            due,
        });
    }

    // Activity, filled in day by day so the chart has no invisible gaps.
    let by_date: HashMap<NaiveDate, &db::DayActivity> =
        daily.iter().map(|d| (d.date, d)).collect();
    let mut activity = Vec::new();
    for offset in (0..ACTIVITY_DAYS).rev() {
        let date = today - chrono::Duration::days(offset);
        let entry = by_date.get(&date);
        activity.push(DayPoint {
            date: date.to_string(),
            count: entry.map(|d| d.reviews).unwrap_or(0),
            accuracy: entry.and_then(|d| {
                (d.total > 0).then(|| d.correct as f32 / d.total as f32)
            }),
        });
    }

    let due_by_date: HashMap<NaiveDate, i64> = forecast_rows.into_iter().collect();
    let overdue: i64 = due_by_date
        .iter()
        .filter(|(date, _)| **date <= today)
        .map(|(_, n)| *n)
        .sum();
    let never_studied = states.iter().filter(|s| s.due.is_none()).count() as i64;
    let mut forecast = Vec::new();
    for offset in 0..FORECAST_DAYS {
        let date = today + chrono::Duration::days(offset);
        // Everything overdue and everything never studied is waiting today.
        let count = if offset == 0 {
            overdue + never_studied
        } else {
            due_by_date.get(&date).copied().unwrap_or(0)
        };
        forecast.push(DayPoint {
            date: date.to_string(),
            count,
            accuracy: None,
        });
    }

    // Hardest topics: lowest accuracy first, ties broken by how much evidence
    // there is, so one bad sheet does not outrank a long struggle.
    let mut hardest: Vec<TopicScore> = records
        .iter()
        .filter_map(|record| {
            let topic = state.pack.topic(&record.topic_id)?;
            let state_row = states.iter().find(|s| s.topic_id == record.topic_id);
            Some(TopicScore {
                id: topic.id.clone(),
                title: topic.title.clone(),
                cefr: topic.cefr,
                accuracy: if record.total > 0 {
                    record.correct as f32 / record.total as f32
                } else {
                    0.0
                },
                reviews: record.reviews,
                lapses: state_row.map(|s| s.lapses).unwrap_or(0),
            })
        })
        .collect();
    hardest.sort_by(|a, b| {
        a.accuracy
            .partial_cmp(&b.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.reviews.cmp(&a.reviews))
    });
    hardest.truncate(TOP_N);

    // A year of days, wound back to a Monday so the heatmap starts a clean week.
    let mut year_start = today - chrono::Duration::days(YEAR_DAYS);
    while year_start.weekday() != Weekday::Mon {
        year_start -= chrono::Duration::days(1);
    }
    let mut year = Vec::new();
    let mut day = year_start;
    let mut studied_days = 0i64;
    while day <= today {
        let entry = by_date.get(&day);
        if entry.is_some() {
            studied_days += 1;
        }
        year.push(DayPoint {
            date: day.to_string(),
            count: entry.map(|d| d.reviews).unwrap_or(0),
            accuracy: entry.and_then(|d| (d.total > 0).then(|| d.correct as f32 / d.total as f32)),
        });
        day += chrono::Duration::days(1);
    }

    let today_row = by_date.get(&today);
    let blanks_correct: i64 = daily.iter().map(|d| d.correct).sum();
    let blanks_total: i64 = daily.iter().map(|d| d.total).sum();

    Ok(Json(Stats {
        today: today.to_string(),
        topics_total: state.pack.topics.len() as i64,
        topics_studied: records.len() as i64,
        topics_due: states.iter().filter(|s| s.is_due(today)).count() as i64,
        reviews_total: daily.iter().map(|d| d.reviews).sum(),
        blanks_total,
        blanks_correct,
        accuracy: if blanks_total > 0 {
            blanks_correct as f32 / blanks_total as f32
        } else {
            0.0
        },
        streak_days: db::streak_from(
            &daily.iter().map(|d| d.date).collect::<Vec<_>>(),
            today,
        ),
        by_level,
        activity,
        forecast,
        weakest: tags
            .into_iter()
            .take(TOP_N)
            .map(|(tag, count)| Weakness { tag, count })
            .collect(),
        hardest,
        longest_streak: db::longest_streak_from(
            &daily.iter().map(|d| d.date).collect::<Vec<_>>(),
        ),
        days_learned: if year.is_empty() {
            0.0
        } else {
            studied_days as f32 / year.len() as f32
        },
        today_reviews: today_row.map(|d| d.reviews).unwrap_or(0),
        today_blanks: today_row.map(|d| d.total).unwrap_or(0),
        today_ms: today_row.map(|d| d.elapsed_ms).unwrap_or(0),
        year,
    }))
}

// ---------------------------------------------------------------- sheets

/// A sheet as the browser sees it: the same sentences with the answers removed.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct ClientSheet {
    #[ts(type = "number")]
    pub sheet_id: i64,
    pub topic_id: String,
    pub topic_title: String,
    pub topic_category: String,
    pub cefr: Level,
    pub items: Vec<ClientItem>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct ClientItem {
    pub n: u32,
    pub segments: Vec<ClientSegment>,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub enum ClientSegment {
    Text { text: String },
    Blank { id: String },
}

fn seed_path(state: &SharedState, topic_id: &str) -> std::path::PathBuf {
    state
        .packs_root
        .join(&state.pack.language)
        .join("sheets")
        .join(format!("{topic_id}.json"))
}

/// Serve the sheet for a topic, creating it from the seed fixture the first time.
async fn sheet_for_topic(
    State(state): State<SharedState>,
    Path(topic_id): Path<String>,
) -> ApiResult<ClientSheet> {
    Ok(Json(load_client_sheet(&state, &topic_id).await?))
}

async fn load_client_sheet(state: &SharedState, topic_id: &str) -> Result<ClientSheet, ApiError> {
    let topic_id = topic_id.to_string();
    let topic = state
        .pack
        .topic(&topic_id)
        .ok_or_else(|| ApiError::not_found(format!("no topic `{topic_id}`")))?;

    let stored = db::latest_sheet_for(&state.db, &topic_id).await?;
    let (sheet_id, sheet) = match stored {
        Some(found) => found,
        None => {
            let path = seed_path(&state, &topic_id);
            let raw = std::fs::read_to_string(&path).map_err(|_| {
                ApiError::not_found(format!("no sheet for `{topic_id}` yet"))
            })?;
            let sheet: Sheet = serde_json::from_str(&raw)?;
            // Invariant: nothing invalid ever reaches the database.
            sheet.validate()?;
            let id = db::save_sheet(&state.db, &sheet, today()).await?;
            (id, sheet)
        }
    };

    Ok(ClientSheet {
        sheet_id,
        topic_id: sheet.topic_id.clone(),
        topic_title: topic.title.clone(),
        topic_category: topic.category.clone(),
        cefr: topic.cefr,
        items: sheet
            .items
            .iter()
            .map(|item| ClientItem {
                n: item.n,
                hint: item.hint.clone(),
                segments: item
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        Segment::Text { text } => ClientSegment::Text { text: text.clone() },
                        Segment::Blank(b) => ClientSegment::Blank { id: b.id.clone() },
                    })
                    .collect(),
            })
            .collect(),
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct CheckRequest {
    /// Blank id to what the user typed.
    pub answers: HashMap<String, String>,
    /// Overrides the rating computed from the score.
    pub rating_override: Option<Rating>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct CheckResponse {
    #[ts(type = "unknown")]
    pub graded: GradedSheet,
    pub rating: Rating,
    #[ts(type = "number")]
    pub interval_days: i64,
    pub due: String,
}

async fn check_sheet(
    State(state): State<SharedState>,
    Path(sheet_id): Path<i64>,
    Json(body): Json<CheckRequest>,
) -> ApiResult<CheckResponse> {
    let sheet = db::load_sheet(&state.db, sheet_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("no sheet {sheet_id}")))?;

    let graded: GradedSheet = grade_sheet(&sheet, &body.answers);
    let rating = body.rating_override.unwrap_or(graded.rating);

    let today = today();
    let state_row = db::topic_state(&state.db, &sheet.topic_id).await?;
    let memory = state_row.as_ref().and_then(|s| s.memory);
    let elapsed = state_row
        .as_ref()
        .map(|s| s.days_since_review(today))
        .unwrap_or(0);

    let scheduled = state.scheduler.review(memory, elapsed, rating, today)?;

    db::record_review(
        &state.db,
        sheet_id,
        &sheet.topic_id,
        &graded,
        rating,
        scheduled.memory,
        scheduled.due,
        today,
        // Sheet mode does not measure time.
        0,
    )
    .await?;

    Ok(Json(CheckResponse {
        graded,
        rating,
        interval_days: scheduled.interval_days,
        due: scheduled.due.to_string(),
    }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct AcceptRequest {
    pub blank_id: String,
    pub answer: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct AcceptResponse {
    pub blank_id: String,
    pub accept: Vec<String>,
    /// The verdict the answer gets now, which should be correct.
    #[ts(type = "unknown")]
    pub verdict: Verdict,
}

/// Overrule the grader. The patch is permanent, so the same answer is right
/// every time this sheet comes back.
async fn accept_also(
    State(state): State<SharedState>,
    Path(sheet_id): Path<i64>,
    Json(body): Json<AcceptRequest>,
) -> ApiResult<AcceptResponse> {
    let mut sheet = db::load_sheet(&state.db, sheet_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("no sheet {sheet_id}")))?;

    if sheet.blank(&body.blank_id).is_none() {
        return Err(ApiError::not_found(format!(
            "sheet {sheet_id} has no blank `{}`",
            body.blank_id
        )));
    }

    sheet.accept_also(&body.blank_id, &body.answer);
    db::update_sheet(&state.db, sheet_id, &sheet).await?;

    let blank = sheet.blank(&body.blank_id).expect("checked above").clone();
    let item = sheet.item_of(&body.blank_id).expect("checked above");
    let verdict = grade_blank(&blank, item, &body.answer);

    Ok(Json(AcceptResponse {
        blank_id: body.blank_id,
        accept: blank.accept,
        verdict,
    }))
}

// ---------------------------------------------------------------- review mode

/// The next thing to study, or nothing when the queue is empty.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct ReviewQueue {
    pub sheet: Option<ClientSheet>,
    /// Topics still due after this one.
    #[ts(type = "number")]
    pub remaining: i64,
}

/// One item as the browser reports it.
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct ItemAnswer {
    pub n: u32,
    /// Milliseconds from the sentence appearing to the answer being sent.
    pub elapsed_ms: u32,
    /// Blank id to what was typed.
    pub answers: HashMap<String, String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct ItemVerdict {
    pub correct: bool,
    /// What this single item earned, before the run is averaged.
    pub grade: Rating,
    #[ts(type = "unknown")]
    pub results: Vec<BlankResult>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct FinishRequest {
    pub items: Vec<ItemAnswer>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../web/src/lib/types/")]
pub struct FinishResponse {
    pub topic_title: String,
    pub rating: Rating,
    #[ts(type = "number")]
    pub interval_days: i64,
    pub due: String,
    #[ts(type = "number")]
    pub correct: usize,
    #[ts(type = "number")]
    pub total: usize,
    pub accuracy: f32,
    /// Topics still due after this one.
    #[ts(type = "number")]
    pub remaining: i64,
}

/// Pick the next due topic in teaching order and hand back its sheet.
async fn review_next(State(state): State<SharedState>) -> ApiResult<ReviewQueue> {
    let views = topic_views(&state).await?;
    let due: Vec<&TopicView> = views.iter().filter(|v| v.is_due && v.has_sheet).collect();

    let Some(next) = due.first() else {
        return Ok(Json(ReviewQueue {
            sheet: None,
            remaining: 0,
        }));
    };

    let sheet = load_client_sheet(&state, &next.id).await?;
    Ok(Json(ReviewQueue {
        sheet: Some(sheet),
        remaining: due.len() as i64 - 1,
    }))
}

/// Grade one item as it is answered, so the reader gets feedback immediately.
///
/// Nothing is stored here. The run is recorded once, at the end, from the same
/// raw answers and timings, so the server stays the authority on the result.
async fn review_item(
    State(state): State<SharedState>,
    Path(sheet_id): Path<i64>,
    Json(body): Json<ItemAnswer>,
) -> ApiResult<ItemVerdict> {
    let sheet = db::load_sheet(&state.db, sheet_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("no sheet {sheet_id}")))?;
    let item = sheet
        .items
        .iter()
        .find(|i| i.n == body.n)
        .ok_or_else(|| ApiError::not_found(format!("sheet {sheet_id} has no item {}", body.n)))?;

    let (results, correct) = grade_one_item(item, &body.answers);
    Ok(Json(ItemVerdict {
        correct,
        grade: grade_item(item, body.elapsed_ms, correct),
        results,
    }))
}

/// Finish a run: regrade everything, rate the topic, move its schedule on.
async fn review_finish(
    State(state): State<SharedState>,
    Path(sheet_id): Path<i64>,
    Json(body): Json<FinishRequest>,
) -> ApiResult<FinishResponse> {
    let sheet = db::load_sheet(&state.db, sheet_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("no sheet {sheet_id}")))?;
    let topic = state
        .pack
        .topic(&sheet.topic_id)
        .ok_or_else(|| ApiError::not_found(format!("no topic `{}`", sheet.topic_id)))?;

    let mut results: Vec<BlankResult> = Vec::new();
    let mut item_ratings = Vec::new();
    let mut elapsed_ms: i64 = 0;

    for attempt in &body.items {
        let Some(item) = sheet.items.iter().find(|i| i.n == attempt.n) else {
            continue;
        };
        let (mut item_results, correct) = grade_one_item(item, &attempt.answers);
        results.append(&mut item_results);
        item_ratings.push(grade_item(item, attempt.elapsed_ms, correct));
        elapsed_ms += attempt.elapsed_ms as i64;
    }

    let total = results.len();
    let correct = results.iter().filter(|r| r.verdict.is_correct()).count();
    let score = if total == 0 {
        0.0
    } else {
        correct as f32 / total as f32
    };
    let graded = GradedSheet {
        results,
        correct,
        total,
        score,
        rating: rating_for(score),
    };

    // Speed and mistakes decide the rating here, not the raw score.
    let rating = session_rating(&item_ratings);

    let today = today();
    let state_row = db::topic_state(&state.db, &sheet.topic_id).await?;
    let memory = state_row.as_ref().and_then(|s| s.memory);
    let days = state_row
        .as_ref()
        .map(|s| s.days_since_review(today))
        .unwrap_or(0);
    let scheduled = state.scheduler.review(memory, days, rating, today)?;

    db::record_review(
        &state.db,
        sheet_id,
        &sheet.topic_id,
        &graded,
        rating,
        scheduled.memory,
        scheduled.due,
        today,
        elapsed_ms,
    )
    .await?;

    let views = topic_views(&state).await?;
    let remaining = views.iter().filter(|v| v.is_due && v.has_sheet).count() as i64;

    Ok(Json(FinishResponse {
        topic_title: topic.title.clone(),
        rating,
        interval_days: scheduled.interval_days,
        due: scheduled.due.to_string(),
        correct,
        total,
        accuracy: score,
        remaining,
    }))
}

/// Grade every blank in one item. The item counts as right only if all of them do.
fn grade_one_item(
    item: &crate::sheet::Item,
    answers: &HashMap<String, String>,
) -> (Vec<BlankResult>, bool) {
    let mut results = Vec::new();
    let mut correct = true;
    for blank in item.blanks() {
        let given = answers.get(&blank.id).cloned().unwrap_or_default();
        let verdict = grade_blank(blank, item, &given);
        if !verdict.is_correct() {
            correct = false;
        }
        results.push(BlankResult {
            blank_id: blank.id.clone(),
            given,
            verdict,
        });
    }
    (results, correct)
}
