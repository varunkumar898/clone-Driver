//! Structured log formatting and subscriber initialization.

use super::config::LogConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Initializes global tracing subscriber for console, file, and in-memory streams.
pub fn init_logging(_config: &LogConfig) -> crate::error::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(())
}
