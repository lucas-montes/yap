use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::comparaison::ComparaisonTechnique;
use super::file::{FileFacade, FileFacadeFactory, Logbook};
use crate::remote::{pull_file, Storage};
use crate::{
    config::Config,
    enums::{ColorWhen, Events},
    remote::PushStrategy,
};

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct DataArgs {
    #[command(subcommand)]
    pub command: DataCommands,
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
pub enum DataCommands {
    /// Start keeping track of one or many files
    #[command(arg_required_else_help = true)]
    Add(AddData),

    /// Commit of one or many files
    #[command(arg_required_else_help = true)]
    Commit(CommitData),

    /// Push the latest version of the selected files
    #[command(arg_required_else_help = true)]
    Push(PushData),

    /// remove one object
    #[command(arg_required_else_help = true)]
    Remove(RemoveData),

    /// Read one or more objects
    #[command(arg_required_else_help = true)]
    Pull(PullData),

    Show(ShowData),
}

impl DataCommands {
    pub async fn handle_commands(&self) -> i16 {
        let config = Config::new();
        match self {
            DataCommands::Add(args) => args.run(&config).await,
            DataCommands::Commit(args) => args.run(&config).await,
            DataCommands::Push(args) => args.run(&config).await,
            DataCommands::Pull(args) => args.run(&config).await,
            DataCommands::Remove(args) => args.run(&config).await,
            DataCommands::Show(args) => args.run(&config).await,
        };
        0
    }
}

async fn get_git_branch() -> String {
    //TODO: rename this function
    match Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
    {
        Ok(v) => String::from_utf8(v.stdout).unwrap().trim().to_owned(),
        Err(err) => {
            println!("{}", err);
            "master".to_string()
        }
    }
}

//TODO: how to manage branches? manage branches like git where you only can have only one at the
//time or do we want to allow the branch to be passed when doing an operation or with git_sync
#[derive(Debug, Args, Clone)]
pub struct AddData {
    #[arg(short, long,num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,
}

impl AddData {
    async fn run(&self, config: &Config) -> i16 {
        let mut files = FileFacadeFactory::new(
            self.paths.clone(),
            &self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        );
        let root_logbook = Logbook::local(&config.local_db()).await;

        while let Some(mut f) = files.anext().await {
            self.handle_file(&mut f, &root_logbook).await;
        }
        0
    }

    async fn handle_file(&self, file: &FileFacade, root_logbook: &Logbook) {
        if !root_logbook.file_is_tracked(file).await {
            let file = file.keep_track().await;
            root_logbook.track_file(file).await;
            root_logbook.save_event(file, &Events::Add).await;
        };
    }
}

#[derive(Debug, Args, Clone)]
pub struct CommitData {
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

impl CommitData {
    async fn run(&self, config: &Config) -> i16 {
        let mut files = FileFacadeFactory::new(
            self.paths.clone(),
            &self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_comparaison(&self.comparaison, &self.script);
        let root_logbook = Logbook::local(&config.local_db()).await;

        while let Some(mut f) = files.anext().await {
            self.handle_file(&mut f, &root_logbook).await;
        }
        0
    }

    async fn handle_file(&self, file: &mut FileFacade, root_logbook: &Logbook) {
        if file.commit(&self.message).await.has_changed() {
            //TODO: save the comparaison results
            root_logbook.save_event(file, &Events::Commit).await;
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct PushData {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, value_enum, required = false)]
    remote: Option<Storage>,

    #[arg(short, long, value_enum, default_value = None, required = false)]
    strategy: Option<PushStrategy>,
}

impl PushData {
    async fn run(&self, config: &Config) -> i16 {
        let mut files = FileFacadeFactory::new(
            self.paths.clone(),
            &self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &self.strategy);
        let root_logbook = Logbook::local(&config.local_db()).await;

        while let Some(mut f) = files.anext().await {
            self.handle_file(&mut f, &root_logbook).await;
        }
        0
    }

    async fn handle_file(&self, file: &mut FileFacade, root_logbook: &Logbook) {
        let file = file.push().await;
        //TODO: push the comparaison results
        //TODO: it would be cool to save the errors if any from the copies and so on and save them
        // in the db
        root_logbook.save_event(file, &Events::Push).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct RemoveData {
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

impl RemoveData {
    async fn run(&self, config: &Config) -> i16 {
        let mut files = FileFacadeFactory::new(
            self.paths.clone(),
            &self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &None);
        let root_logbook = Logbook::local(&config.local_db()).await;

        while let Some(mut f) = files.anext().await {
            self.handle_file(&mut f, &root_logbook).await;
        }
        0
    }

    async fn handle_file(&self, file: &mut FileFacade, root_logbook: &Logbook) {
        let file = file.remove().await;
        root_logbook.save_event(file, &Events::Remove).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct PullData {
    #[arg(short, long, num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, value_enum)]
    remote: Option<Storage>,
}

impl PullData {
    async fn run(&self, config: &Config) -> i16 {
        let mut files = FileFacadeFactory::new(
            self.paths.clone(),
            &self.branch.as_ref().unwrap_or(&get_git_branch().await),
            config,
        )
        .set_remote(config, &self.remote, &None);
        let root_logbook = Logbook::local(&config.local_db()).await;

        while let Some(mut f) = files.anext().await {
            self.handle_file(&mut f, &root_logbook).await;
        }
        0
    }

    async fn handle_file(&self, file: &mut FileFacade, root_logbook: &Logbook) {
        pull_file(file).await;
        root_logbook.save_event(file, &Events::Pull).await;
    }
}

#[derive(Debug, Args, Clone)]
pub struct ShowData {
    #[arg(short, long, num_args = 0..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, required = false)]
    branch: Option<String>,

    #[arg(short, long, required = false)]
    commit: Option<String>,
}

impl ShowData {
    async fn run(&self, config: &Config) -> i16 {
        let root_logbook = Logbook::local(&config.local_db()).await;
        let data = root_logbook.files_tracked().await;
        self.paging(data.join("\n").as_bytes()).await;
        0
    }

    async fn paging(&self, data: &[u8]){
        let mut child = Command::new("less").stdin(Stdio::piped()).spawn().unwrap();

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(data).await.unwrap();
        }
        child.wait().await.unwrap();
    }
}
