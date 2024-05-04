use super::{
    comparaison::ComparaisonTechnique,
    file::{FileFacade, FileFacadeFactory, Logbook},
};
use crate::{
    config::{Config, PushStrategy, Storage},
    enums::ColorWhen,
};

use futures::{stream::FuturesUnordered, StreamExt};
use std::process::Stdio;
use std::{fmt, path::PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use clap::{Args, Subcommand};

#[derive(Debug)]
pub enum Events {
    Add,
    Commit,
    Pull,
    Push,
    Remove,
}

impl fmt::Display for Events {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct VcsArgs {
    #[command(subcommand)]
    pub command: VcsCommands,
    #[arg(
        long,
        require_equals = true,
        value_name = "WHEN",
        num_args = 0..=1,
        default_value_t = ColorWhen::Auto,
        default_missing_value = "always",
        value_enum
    )]
    color: ColorWhen,
}

#[derive(Debug, Subcommand)]
pub enum VcsCommands {
    /// Start keeping track of one or many files
    #[command(arg_required_else_help = true)]
    Add(Add),

    /// Commit of one or many files
    #[command(arg_required_else_help = true)]
    Commit(Commit),

    /// Push the latest version of the selected files
    #[command(arg_required_else_help = true)]
    Push(Push),

    /// remove one object
    #[command(arg_required_else_help = true)]
    Remove(Remove),

    /// Read one or more objects
    #[command(arg_required_else_help = true)]
    Pull(Pull),

    Show(Show),
}

impl VcsCommands {
    pub async fn handle_commands(&self) -> i16 {
        let config = Config::new();
        match self {
            VcsCommands::Add(args) => args.run(&config).await,
            VcsCommands::Commit(args) => args.run(&config).await,
            VcsCommands::Push(args) => args.run(&config).await,
            VcsCommands::Pull(args) => args.run(&config).await,
            VcsCommands::Remove(args) => args.run(&config).await,
            VcsCommands::Show(args) => args.run(&config).await,
        };
        0
    }
}

async fn get_git_branch() -> String {
    //TODO: rename this function
    match Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
    {
        Ok(v) => String::from_utf8(v.stdout).unwrap().trim().to_owned(),
        Err(err) => {
            println!("{}", err);
            "master".to_string()
        },
    }
}

trait Vcs {
    async fn run(&self, config: &Config) -> i16 {
        let files = self.get_files_factory(config).await;
        let root_logbook = Logbook::local(&config.local_db()).await;
        let tasks: FuturesUnordered<_> = FuturesUnordered::new();
        for file in files {
            tasks.push(self.initialize_file_facade(file, &root_logbook));
        }
        let _: Vec<()> = tasks.collect().await;
        0
    }
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory;
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook);
    async fn initialize_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        self.handle_file_facade(file.init().await, root_logbook)
            .await;
    }
}

//TODO: how to manage branches? manage branches like git where you only can have only one at the
//time or do we want to allow the branch to be passed when doing an operation or with git_sync
#[derive(Debug, Args, Clone)]
pub struct Add {
    #[arg(short, long,num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,
}

impl Vcs for Add {
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory {
        FileFacadeFactory::new(
            self.paths.clone(),
            self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
    }
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        if !root_logbook.file_is_tracked(&file).await {
            let file = file.add().await;
            root_logbook.track_file(&file).await;
            root_logbook.save_event(&file, &Events::Add).await;
        };
    }
}

#[derive(Debug, Args, Clone)]
pub struct Commit {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long)]
    message: String,

    #[arg(short, long, default_value = "smart", value_enum)]
    comparaison: ComparaisonTechnique,

    // Path to the script to execute when the comparaison technique is Custom
    #[arg(short, long, required = false)]
    script: Option<PathBuf>,
}
impl Vcs for Commit {
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory {
        FileFacadeFactory::new(
            self.paths.clone(),
            self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_message(&self.message)
        .set_comparaison(&self.comparaison, &self.script)
    }
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        if file.compare(&self.message).await.has_changed() {
            //TODO: save the comparaison results
            root_logbook.save_event(&file, &Events::Commit).await;
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct Push {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, value_enum, required = false)]
    remote: Option<Storage>,

    #[arg(short, long, value_enum, default_value = None, required = false)]
    strategy: Option<PushStrategy>,

    #[arg(short, long, default_value_t = true)]
    compress: bool,
}
impl Vcs for Push {
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory {
        FileFacadeFactory::new(
            self.paths.clone(),
            self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &self.strategy)
    }
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        let file = file.push().await;
        root_logbook.save_event(&file, &Events::Push).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct Remove {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, required = false)]
    //TODO: do I want to delete by id?
    // or even timestamp?
    commit: Option<u32>,

    #[arg(short, long, value_enum, required = false)]
    remote: Option<Storage>,

    // Remove the files permanently. Meaning that it not only remove them from the source control
    // but also deletes them on the remote storage.
    // TODO: convert this into an enum Local, Remote, All
    #[arg(short, long, default_value_t = false, required = false)]
    which: bool,
}

impl Vcs for Remove {
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory {
        FileFacadeFactory::new(
            self.paths.clone(),
            self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &None)
    }
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        let file = file.remove().await;
        root_logbook.save_event(&file, &Events::Remove).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct Pull {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, value_enum)]
    remote: Option<Storage>,
}

impl Vcs for Pull {
    async fn get_files_factory(&self, config: &Config) -> FileFacadeFactory {
        FileFacadeFactory::new(
            self.paths.clone(),
            self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &None)
    }
    async fn handle_file_facade(&self, file: FileFacade, root_logbook: &Logbook) {
        let file = file.pull().await;
        root_logbook.save_event(&file, &Events::Pull).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct Show {
    #[arg(short, long, num_args = 0..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, required = false)]
    commit: Option<String>,
}

impl Show {
    async fn run(&self, config: &Config) -> i16 {
        let root_logbook = Logbook::local(&config.local_db()).await;
        let data = root_logbook.files_tracked().await;
        self.paging(data.join("\n").as_bytes()).await;
        0
    }

    async fn paging(&self, data: &[u8]) {
        let mut child = Command::new("less").stdin(Stdio::piped()).spawn().unwrap();

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(data).await.unwrap();
        }
        child.wait().await.unwrap();
    }
}
