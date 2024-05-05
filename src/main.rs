#[cfg(feature = "documentation")]
mod documentation;
#[cfg(feature = "knowbase")]
mod knowbase;
#[cfg(feature = "repro")]
mod repro;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "todo")]
mod todo;
#[cfg(feature = "vcs")]
mod vcs;

mod cli;
mod config;
mod enums;
mod utils;
use cli::Cli;

fn main() -> Result<(), i16> {
    tokio_uring::start(async {
        Cli::handle().await;
        Ok(())
    })
}
