//! lacuna backend.
//!
//! See `CLAUDE.md` at the repo root for the invariants this code has to keep.

pub mod api;
pub mod db;
pub mod domain;
pub mod grade;
pub mod pack;
pub mod review;
pub mod schedule;
pub mod sheet;

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::pack::Pack;
use crate::schedule::Scheduler;

/// Everything the request handlers share.
pub struct AppState {
    pub db: SqlitePool,
    pub pack: Pack,
    pub scheduler: Scheduler,
    /// Root of the `packs/` directory, used to find seed sheets.
    pub packs_root: PathBuf,
}

pub type SharedState = Arc<AppState>;
