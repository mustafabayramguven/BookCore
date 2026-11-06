use crate::config::AppConfig;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: DbPool,
}

impl AppState {
    pub fn new(config: AppConfig, pool: DbPool) -> Self {
        Self { config, pool }
    }
}
