//! lacuna server entry point.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use lacuna_server::{api, db, pack::Pack, schedule::Scheduler, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "lacuna_server=info,tower_http=warn".into()),
        )
        .init();

    let packs_root = std::env::var("LACUNA_PACKS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_packs_root());
    let language = std::env::var("LACUNA_LANGUAGE").unwrap_or_else(|_| "de".to_string());
    let database = std::env::var("LACUNA_DB").unwrap_or_else(|_| "sqlite://lacuna.db".to_string());
    let port: u16 = std::env::var("LACUNA_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4000);

    // A broken pack stops the server here rather than halfway through a session.
    let pack = Pack::load(&packs_root, &language)?;
    tracing::info!(
        "loaded pack `{}` with {} topics",
        pack.language,
        pack.topics.len()
    );

    let db = db::connect(&database).await?;
    db::sync_pack(&db, &pack).await?;

    let state = Arc::new(AppState {
        db,
        pack,
        scheduler: Scheduler::default(),
        packs_root,
    });

    let app = api::router(state).layer(tower_http::cors::CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("lacuna listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// `packs/` sits next to the `server/` crate in the repo.
fn default_packs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../packs")
}
