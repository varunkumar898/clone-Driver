//! DiskClone Core Library
//!
//! A high-performance, safety-critical Linux disk cloning application.

#![allow(dead_code)]

pub mod commands;
pub mod config;
pub mod confirm;
pub mod device;
pub mod error;
pub mod linux;
pub mod logging;
pub mod partition;
pub mod privilege;
pub mod resume;
pub mod safety;
pub mod state;
pub mod state_machine;
pub mod storage;
pub mod ui;

pub use config::AppConfig;
pub use error::{CloneError, Result};
