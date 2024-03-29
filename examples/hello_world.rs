use std::env;

use anyhow::{Ok, Result};
use dotenvy::dotenv;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use rymo::http::Request;
use rymo::{
    http::{self},
    Response, Rymo,
};

pub fn init_logger() {
    let formatting_layer = fmt::layer()
        // .pretty()
        .with_thread_ids(false)
        .with_target(false)
        .with_writer(std::io::stdout);

    let env_layer = EnvFilter::try_from_env("RYMO_LOG").unwrap_or_else(|_| "info".into());

    registry().with(env_layer).with(formatting_layer).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT").unwrap_or("4000".into());
    info!("listening on {port}");
    let app = Rymo::new(&port);

    app.get("/", handler).await;
    app.post("/", handler).await;
    app.serve().await?;
    Ok(())
}

async fn handler(req: Request) -> Response {
    Response(http::Status::Ok, req.body.into())
}
