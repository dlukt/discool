pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod static_files;

pub use error::AppError;

use std::sync::Arc;
use std::time::Instant;

use crate::db::DbPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: DbPool,
    pub start_time: Instant,
}
