mod cli;
mod config;
mod data;
mod documentation;
mod enums;
mod repro;
mod versioning;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<(), i16> {
    Cli::handle().await;
    Ok(())
}
