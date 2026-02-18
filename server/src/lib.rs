pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod static_files;

pub use error::AppError;

use std::sync::Arc;

use sqlx::AnyPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: AnyPool,
}
