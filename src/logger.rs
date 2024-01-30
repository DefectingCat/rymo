use std::env;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

pub fn init_logger() {
    let formatting_layer = fmt::layer()
        // .pretty()
        .with_thread_ids(false)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::try_from_env("RHEA_LOG").unwrap_or_else(|_| {
        format!("{}=info,tower_http=info,axum=info", env!("CARGO_PKG_NAME")).into()
    });

    registry().with(env_layer).with(formatting_layer).init();
}