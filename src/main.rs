use anyhow::Result;
use dotenvy::dotenv;
use logger::init_logger;
use rymo::{http, Response, Rymo};
use std::env;
use tracing::{info, warn};

mod logger;

fn test() -> Response {
    Box::pin(async move { (http::Status::Ok, "Hello Rymo from POST method".into()) })
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT").unwrap_or("4000".into());
    info!("listening on {port}");
    let app = Rymo::new(&port);

    app.get(
        "/",
        Box::new(|| {
            Box::pin(async move { (http::Status::Ok, "Hello Rymo from GET method".into()) })
        }),
    )
    .await;
    app.get(
        "/test",
        Box::new(|| {
            Box::pin(async move { (http::Status::Ok, "Hello test from GET method".into()) })
        }),
    )
    .await;
    app.post("/", Box::new(test)).await;
    app.serve().await?;
    Ok(())
}
