use std::path::{Path, PathBuf};

use crate::config::ConfigArgs;
use crate::data::DataArgs;
use crate::documentation::DocsArgs;
use crate::repro::{repro, ReproArgs};

use clap::Parser;
use menva::read_env_file;

#[derive(Debug, Parser)]
#[command(name = "Yap", version = "0.0.1")]
#[command(about = "Manage your data and documentation")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn handle() -> i16 {
        read_env_file(".env");
        match &Cli::parse().command {
            Commands::Data(args) => args.command.handle_commands().await,
            Commands::Docs(args) => args.command.handle_commands(),
            Commands::Config(args) => args.command.handle_commands().await,
            Commands::Repro(args) => repro(args).await,
            Commands::Test => {
println!("{:?}", PathBuf::from("/").canonicalize().unwrap());
                todo!()
            },
        }
    }
}

#[derive(Debug, Parser)]
enum Commands {
    Test,
    /// Version control your data
    #[command(arg_required_else_help = true)]
    Config(ConfigArgs),

    /// Version control your data
    #[command(arg_required_else_help = true)]
    Data(DataArgs),

    /// Reproduce a job
    #[command(arg_required_else_help = true)]
    Repro(ReproArgs),

    /// Version control your docs
    #[command(arg_required_else_help = true)]
    Docs(DocsArgs),
}
