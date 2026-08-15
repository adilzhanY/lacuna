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
