use anyhow::Result;
use dotenvy::dotenv;
use logger::init_logger;
use rymo::Rymo;
use std::env;
use tracing::{info, warn};

mod logger;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT").unwrap_or("4000".into());
    info!("listening on {port}");
    let mut app = Rymo::new(&port);
    app.get("/", || async move {});
    app.serve().await?;
    Ok(())
}
