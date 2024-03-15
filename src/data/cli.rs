use futures::stream::StreamExt;

use std::path::PathBuf;
use std::sync::Arc;

use super::comparaison::compare;
use crate::config::Config;
use crate::enums::{ColorWhen, Events};
use super::file::{FileFacade, FileFacadeFactory, Logbook};

use clap::{Args, Subcommand};
use opendal::services::Gcs;
use opendal::Operator;

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
    Commit(CommitData),

    /// Push the latest version of the selected files
    Push(PushData),

    /// remove one object
    #[command(arg_required_else_help = true)]
    Remove(RemoveData),

    /// Read one or more objects
    #[command(arg_required_else_help = true)]
    Get(GetData),
}

impl DataCommands {
    fn create_operator() -> Operator {
        // create backend builder
        let mut builder = Gcs::default();

        // set the storage bucket for OpenDAL
        builder.bucket("test");
        // set the working directory root for GCS
        // all operations will happen within it
        builder.root("/path/to/dir");
        // set the credential of service account.
        builder.credential("service account credential");
        // set the predefined ACL for GCS
        builder.predefined_acl("publicRead");
        // set the default storage class for GCS
        builder.default_storage_class("STANDARD");

        match Operator::new(builder) {
            Ok(op) => op.finish(),
            Err(err) => panic!("{:?}", err),
        }
    }
    pub async fn handle_commands(&self) -> i16 {
        let config = Config::new();
        let op = Self::create_operator();
        match self {
            DataCommands::Add(args) => args.run(&config).await,
            DataCommands::Commit(args) => args.run(&config).await,
            DataCommands::Push(args) => args.run(&op).await,
            DataCommands::Get(args) => args.run(&op).await,
            DataCommands::Remove(args) => args.run(&op).await,
        };
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct AddData {
    #[arg(short, long,num_args = 1..)]
    paths: Vec<PathBuf>,

    #[arg(short, long, default_value = "master")]
    branch: String,
}

impl AddData {
    async fn run(&self, config: &Config) -> i16 {
        let files =
            FileFacadeFactory::new(self.paths.clone(), &self.branch, &config.logbooks_dir());
        let user_logbook = Arc::new(Logbook::local(&config.local_db()));

        futures::stream::iter(files)
            .for_each(|f| {
                let user_logbook = Arc::clone(&user_logbook);
                async move {
                    self.handle_file(&f, user_logbook, &config.history_dir())
                        .await;
                }
            })
            .await;
        0
    }

    async fn handle_file(&self, file: &FileFacade, user_logbook: Arc<Logbook>, history: &str) {
        if !user_logbook.file_is_tracked(file, &Events::Add).await {
            let file = file.duplicate(history).insert_snapshot().await;
            user_logbook.insert_log(file, &Events::Add).await;
        };
    }
}

#[derive(Debug, Args, Clone)]
pub struct CommitData {
    #[arg(short, long, required = false)]
    paths: Vec<PathBuf>,

    #[arg(short, long, default_value = "master")]
    branch: String,

    #[arg(short, long)]
    message: String,
}

impl CommitData {
    async fn run(&self, config: &Config) -> i16 {
        let files =
            FileFacadeFactory::new(self.paths.clone(), &self.branch, &config.logbooks_dir());
        let user_logbook = Arc::new(Logbook::local(&config.local_db()));

        futures::stream::iter(files)
            .for_each(|f| {
                let user_logbook = Arc::clone(&user_logbook);
                async move {
                    self.handle_file(&f, user_logbook, &config.history_dir())
                        .await;
                }
            })
            .await;
        0
    }

    async fn handle_file(&self, file: &FileFacade, user_logbook: Arc<Logbook>, history: &str) {
        if file.changed() {
            let file = file.duplicate(history).insert_snapshot().await;
            user_logbook.insert_log(file, &Events::Commit).await;
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct RemoveData {
    #[arg(short, long)]
    paths: Vec<PathBuf>,

    // Remove the files permanently. Meaning that it not only remove them from the source control
    // but also deletes them on the remote storage.
    #[arg(short, long, default_value_t = false, required = false)]
    permanently: bool,
}

impl RemoveData {
    async fn run(&self, operator: &Operator) -> i16 {
        match operator
            .remove(
                self.paths
                    .iter()
                    .map(|f| f.to_str().unwrap().to_owned())
                    .collect(),
            )
            .await
        {
            Ok(_) => 0,
            Err(err) => panic!("{:?}", err),
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct GetData {
    #[arg(short, long)]
    paths: Vec<PathBuf>,
}

impl GetData {
    async fn run(&self, operator: &Operator) -> i16 {
        match operator.copy("path/to/file", "path/to/file2").await {
            Ok(_) => 0,
            Err(err) => panic!("{:?}", err),
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct PushData {
    #[arg(short, long)]
    paths: Vec<PathBuf>,
}

impl PushData {
    async fn run(&self, operator: &Operator) -> i16 {
        match operator.copy("path/to/file", "path/to/file2").await {
            Ok(_) => 0,
            Err(err) => panic!("{:?}", err),
        }
    }
}
