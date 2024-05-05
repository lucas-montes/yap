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
use glommio::LocalExecutor;

fn main() -> Result<(), i16> {
    let local_ex = LocalExecutor::default();
    local_ex.run(async {
        Cli::handle().await;
    });

    // LocalExecutorBuilder::new(Placement::Fixed(0))
    //     .spawn(|| async move {
    //         Cli::handle().await;
    //     })
    //     .unwrap()
    //     .join()
    //     .unwrap();
    Ok(())
}
