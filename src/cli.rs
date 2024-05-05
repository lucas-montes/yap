#[cfg(feature = "documentation")]
use crate::documentation::DocsArgs;
#[cfg(feature = "repro")]
use crate::repro::{repro, ReproArgs};
#[cfg(feature = "server")]
use crate::server::ServeArgs;
#[cfg(feature = "todo")]
use crate::todo::utils::cli::ToDoCli;
#[cfg(feature = "vcs")]
use crate::vcs::VcsArgs;

use crate::config::ConfigArgs;

use clap::Parser;
use menva::read_env_file;

#[derive(Debug, Parser)]
#[command(name = "Yap", version = "0.0.0")]
#[command(about = "Manage your data and documentation")]
pub struct Cli {
    #[arg(short, long, required = false)]
    env: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn handle() -> i16 {
        let cli = Cli::parse();
        if let Some(env) = cli.env.as_ref() {
            read_env_file(env);
        }
        match cli.command {
            Commands::Config(args) => args.command.handle_commands().await,
            #[cfg(feature = "vcs")]
            Commands::Vcs(args) => args.command.handle_commands().await,
            #[cfg(feature = "documentation")]
            Commands::Docs(args) => args.command.handle_commands(),
            #[cfg(feature = "repro")]
            Commands::Repro(args) => repro(&args).await,
            #[cfg(feature = "todo")]
            Commands::Todo(args) => args.handle_args().await,
            #[cfg(feature = "lsp")]
            Commands::Lsp => todo!(),
            #[cfg(any(feature = "documentation", feature = "server"))]
            Commands::Serve(args) => args.handle_args().await,
        }
    }
}

#[derive(Debug, Parser)]
enum Commands {
    // Your own ToDo list. Set your goals, projects and tasks so you can enjoy Scrum at home.
    #[cfg(feature = "todo")]
    Todo(ToDoCli),
    // Enable the lsp this will be called from a text editor
    #[cfg(feature = "lsp")]
    Lsp,
    #[cfg(any(feature = "documentation", feature = "server"))]
    /// Launch a server to display documentation or notes
    Serve(ServeArgs),
    #[cfg(feature = "repro")]
    /// Reproduce a job
    #[command(arg_required_else_help = true)]
    Repro(ReproArgs),
    #[cfg(feature = "documentation")]
    /// Manage documentation from various projects
    #[command(arg_required_else_help = true)]
    Docs(DocsArgs),
    /// Tune your configurations. Set up your remote logbook, your email and so on
    #[command(arg_required_else_help = true)]
    Config(Box<ConfigArgs>),
    #[cfg(feature = "vcs")]
    /// Version control your data
    #[command(arg_required_else_help = true)]
    Vcs(VcsArgs),
}
