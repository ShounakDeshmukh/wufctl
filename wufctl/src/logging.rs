use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use chrono::Local;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_error::ErrorLayer;
use tracing_subscriber::fmt;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::utils;

const MAX_LOG_AGE: Duration = Duration::from_secs(5 * 24 * 60 * 60);

pub fn init() -> WorkerGuard {
    let logdir = utils::get_config_dir().expect("Failed to get config directory");
    fs::create_dir_all(&logdir).expect("Failed to create log directory");
    clean_old_logs(&logdir);

    let filename = format!("wufctl-{}.log", Local::now().format("%Y%m%d-%H%M%S"));
    let file_appender = rolling::never(&logdir, filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // RUST_LOG wins if set; otherwise "warn" for dependencies, full debug for our own code.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,wufctl=debug,vcl_lib=debug"));

    // color-eyre needs a Registry with an ErrorLayer to capture a SpanTrace, not a bare fmt::Subscriber.
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking))
        .with(ErrorLayer::default())
        .init();

    guard
}

/// Best-effort - a scan failure or leftover stale log isn't worth failing startup over.
fn clean_old_logs(logdir: &Path) {
    let Ok(entries) = fs::read_dir(logdir) else {
        return;
    };
    let Some(cutoff) = SystemTime::now().checked_sub(MAX_LOG_AGE) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }
        let is_stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|modified| modified < cutoff);
        if is_stale {
            let _ = fs::remove_file(&path);
        }
    }
}
