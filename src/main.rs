mod cli;
mod data;
mod documentation;
mod enums;
mod repro;
mod settings;
mod turso;
mod versioning;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<(), i16> {
    Cli::handle().await;
    Ok(())
}
