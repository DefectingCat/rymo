use std::env;

use anyhow::Result;
use dotenvy::dotenv;
use tracing::{info, warn};

use logger::init_logger;
use rymo::{
    http::{self},
    Rymo,
};

mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT").unwrap_or("4000".into());
    info!("listening on {port}");
    let app = Rymo::new(&port);

    app.post("/", |req| {
        let task = async move {
            (
                http::Status::Ok,
                format!(
                    "Hello Rymo {} method from {}",
                    req.method,
                    req.headers
                        .get("User-Agent")
                        .unwrap_or(&"Unknown".to_string())
                )
                .into(),
            )
        };
        Box::pin(task)
    })
    .await;
    // app.get("/test", test_handler).await;
    app.serve().await?;
    Ok(())
}
