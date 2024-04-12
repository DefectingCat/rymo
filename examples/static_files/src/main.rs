use std::env;
use std::path::PathBuf;

use anyhow::{Ok, Result};
use dotenvy::dotenv;
use rymo::http::request::Request;
use rymo::http::response::Response;
use rymo::Rymo;
use tokio::fs;
use tracing::{info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};

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

async fn handler(_req: Request, mut res: Response) -> Result<Response> {
    let path = PathBuf::from("./public/index.html");
    let index = fs::read(path).await?;
    res.headers.insert(
        "Content-Type".to_owned(),
        "text/html; charset=utf-8".to_owned(),
    );
    res.body = index.into();
    Ok(res)
}
