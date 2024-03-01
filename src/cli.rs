use crate::data::DataArgs;
use crate::documentation::DocsArgs;
use crate::repro::{ReproArgs,repro};
use crate::settings::Settings;
use crate::versioning::run;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "Yap", version = "0.0.1")]
#[command(about = "Manage your data and documentation")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn handle() -> i16 {
        let stdin_args = Cli::parse();
        let settings = Settings::new();
        match &stdin_args.command {
            Commands::Data(args) => args.command.handle_commands(&settings).await,
            Commands::Docs(args) => args.command.handle_commands(),
            Commands::Repro(args) => repro(args).await,
            Commands::Test => run().await,
        }
    }
}

#[derive(Debug, Parser)]
enum Commands {
    Test,
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
