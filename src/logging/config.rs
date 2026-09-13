//! Logging configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub log_to_stdout: bool,
    pub log_to_file: bool,
    pub log_file_path: Option<PathBuf>,
    pub max_in_memory_entries: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_stdout: true,
            log_to_file: true,
            log_file_path: Some(PathBuf::from("/tmp/diskclone.log")),
            max_in_memory_entries: 500,
        }
    }
}
