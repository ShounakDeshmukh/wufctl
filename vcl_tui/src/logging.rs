use std::fs;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_error::ErrorLayer;
use tracing_subscriber::fmt;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::utils;

pub fn init() -> WorkerGuard {
    let logdir = utils::get_config_dir().expect("Failed to get config directory");
    fs::create_dir_all(&logdir).expect("Failed to create log directory");

    let file_appender = rolling::never(&logdir, "vcl_tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // RUST_LOG wins if set; otherwise "warn" for dependencies, full debug for our own code.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,vcl_tui=debug,vcl_lib=debug"));

    // color-eyre needs a Registry with an ErrorLayer to capture a SpanTrace, not a bare fmt::Subscriber.
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking))
        .with(ErrorLayer::default())
        .init();

    guard
}
