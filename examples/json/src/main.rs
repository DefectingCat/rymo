use std::env;

use anyhow::{Ok, Result};
use dotenvy::dotenv;
use rymo::http::request::Request;
use rymo::http::response::Response;
use rymo::Rymo;
use serde_json::json;
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

async fn handler(req: Request, res: Response) -> Result<Response> {
    let unknown = "Unknown".to_owned();
    let host = req.headers.get("host").unwrap_or(&unknown);
    info!("handle request from {host}",);

    let _body = json!({
        "status": "ok",
        "message": "hello world"
    });
    // Response(http::Status::Ok, body.to_string().into())
    Ok(res)
}
