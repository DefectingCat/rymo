use anyhow::Result;
use dotenvy::dotenv;
use logger::init_logger;
use rymo::Rymo;
use std::env;
use tracing::{info, warn};

mod logger;
mod rymo;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    dotenv().map_err(|err| warn!("env file {err}")).ok();

    let port = env::var("PORT")
        .unwrap_or("4000".into())
        .parse()
        .unwrap_or(4000);
    info!("listening on {port}");
    Rymo::listen(port);
    Ok(())
}
