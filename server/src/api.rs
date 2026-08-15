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
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db;
use crate::domain::{Level, Rating};
use crate::grade::{grade_blank, grade_sheet, GradedSheet, Verdict};
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
}

const ACTIVITY_DAYS: i64 = 30;
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

    Ok(Json(ClientSheet {
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
    }))
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
