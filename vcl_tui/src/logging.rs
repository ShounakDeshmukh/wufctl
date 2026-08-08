use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::utils;

pub fn init() -> tracing_appender::non_blocking::WorkerGuard {
    // Check if logdir exists, if not create it
    let logdir = utils::get_config_dir().expect("Failed to get config directory");
    std::fs::create_dir_all(&logdir).expect("Failed to create log directory");

    let file_appender = tracing_appender::rolling::never(&logdir, "vcl_tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // RUST_LOG, when set, always wins. Otherwise default to "warn" for
    // every dependency but full debug for our own code - vcl_lib emits
    // useful log::debug! instrumentation (request/response bodies) via the
    // log bridge that's the whole reason this file log exists; defaulting
    // bare EnvFilter::from_default_env() to no RUST_LOG set would fall
    // back to error-only and silently hide exactly that.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,vcl_tui=debug,vcl_lib=debug"));

    // A bare `fmt().init()` only builds a `fmt::Subscriber`, not a
    // `Registry` with layers - color-eyre needs a `tracing_error::ErrorLayer`
    // registered here so it can capture a SpanTrace when a report is
    // created; without it every color-eyre report shows an empty span
    // trace even with the panic/eyre hooks installed correctly.
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(ErrorLayer::default())
        .init();

    guard
}
