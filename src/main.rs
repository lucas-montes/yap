mod cli;
mod turso_client;
mod versioning;
mod settings;
mod repro;
mod data;
mod documentation;
mod enums;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<(), i16> {
    Cli::handle().await;
    Ok(())
}
