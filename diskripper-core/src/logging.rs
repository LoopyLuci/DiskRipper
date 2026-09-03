//! File logging with rolling rotation for diagnostics.

use std::path::PathBuf;
use std::sync::OnceLock;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Initialize file logging with rolling rotation
pub fn init_file_logging() -> Result<(), crate::error::DiskRipperError> {
    let log_dir = get_log_dir();
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| crate::error::DiskRipperError::Io(e.to_string()))?;

    // Daily rolling file appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "diskripper.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Store guard to keep it alive
    let _ = LOG_GUARD.set(guard);

    // Console layer (stdout)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(EnvFilter::from_default_env());

    // File layer (JSON for machine parsing)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!(log_dir = %log_dir.display(), "File logging initialized");
    Ok(())
}

/// Get the log directory for the current platform
pub fn get_log_dir() -> PathBuf {
    if let Some(data_dir) = dirs::data_dir() {
        data_dir.join("DiskRipper").join("logs")
    } else {
        PathBuf::from("./logs")
    }
}

/// Get the config directory for the current platform
pub fn get_config_dir() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("DiskRipper")
    } else {
        PathBuf::from("./config")
    }
}

/// Get the cache directory for the current platform
pub fn get_cache_dir() -> PathBuf {
    if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join("DiskRipper")
    } else {
        PathBuf::from("./cache")
    }
}
