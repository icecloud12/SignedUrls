use std::sync::Arc;

use mongodb::Database;

use crate::config::Config;

/// Shared state handed to every handler through axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(db: Database, config: Config) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}
