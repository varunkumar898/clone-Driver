//! Structured logging and diagnostics subsystem.

pub mod config;
pub mod format;

pub use config::LogConfig;
pub use format::{init_logging, LogEntry};
