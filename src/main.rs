use anyhow::Result;
use dotenvy::dotenv;
use logger::init_logger;
use rymo::{http, Rymo};
use std::env;
use tracing::{info, warn};

mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT").unwrap_or("4000".into());
    info!("listening on {port}");
    let app = Rymo::new(&port);

    app.get(
        "/",
        Box::new(|req| {
            Box::pin(async move {
                (
                    http::Status::Ok,
                    format!(
                        "Hello Rymo {} method from {}",
                        req.method,
                        req.headers
                            .get("User-Agent")
                            .unwrap_or(&"Unknow".to_string())
                    )
                    .into(),
                )
            })
        }),
    )
    .await;
    app.get(
        "/test",
        Box::new(|_| {
            Box::pin(async move { (http::Status::Ok, "Hello test from GET method".into()) })
        }),
    )
    .await;
    app.serve().await?;
    Ok(())
}
