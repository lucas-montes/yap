use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config::Config;
use crate::enums::ColorWhen;
use crate::versioning::{connect_db, File};

use clap::{Args, Subcommand};
use opendal::services::Gcs;
use opendal::Operator;
use rayon::prelude::*;

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
    fn get_credentials() {}
    fn get_config() {}
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
            DataCommands::Commit(args) => args.run().await,
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
    #[arg(
        short,
        long,
        default_value_t = true,
        required = false,
        help = "Set to true so a copy of the file is created after each commit to keep track of the changes"
    )]
    copy: bool,
}

fn create_path_to_history() -> PathBuf {
    let mut current_dir = std::env::current_dir().unwrap();
    current_dir.push(".yap/history");
    let now = chrono::offset::Local::now().timestamp();
    current_dir.push(format!("{now:?}"));
    fs::create_dir_all(&current_dir).unwrap();
    current_dir
}

impl AddData {
    async fn run(&self, config: &Config) -> i16 {
        //TODO: check if files already exists before running everything
        let root_project = std::env::current_dir().unwrap();
        let history = create_path_to_history();
        let files: Vec<File> = self
            .paths
            .par_iter()
            .flat_map(|p| {
                let mut rp = root_project.clone();
                rp.push(p);
                AddData::handle_path(&rp)
            })
            .collect();
        if self.copy {
            files.par_iter().for_each(|f| f.duplicate(history.clone()));
        }
        let conn = connect_db().await;
        File::add_many(&conn, &files).await;
        0
    }

    fn handle_path(path: &PathBuf) -> Vec<File> {
        if path.is_dir() {
            return fs::read_dir(path)
                .unwrap()
                .flat_map(|p| AddData::handle_path(&p.unwrap().path()))
                .collect();
        }
        let file = File::new(&path);
        vec![file]
    }
}

#[derive(Debug, Args, Clone)]
pub struct CommitData {
    #[arg(short, long, required = false)]
    paths: Vec<String>,
}

impl CommitData {
    async fn run(&self) -> i16 {
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct RemoveData {
    #[arg(short, long)]
    paths: Vec<String>,

    // Remove the files permanently. Meaning that it not only remove them from the source control
    // but also deletes them on the remote storage.
    #[arg(short, long, default_value_t = false, required = false)]
    permanently: bool,
}

impl RemoveData {
    async fn run(&self, operator: &Operator) -> i16 {
        match operator.remove(self.paths.clone()).await {
            Ok(_) => 0,
            Err(err) => panic!("{:?}", err),
        }
    }
}

#[derive(Debug, Args, Clone)]
pub struct GetData {
    #[arg(short, long)]
    paths: Vec<String>,
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
    paths: Vec<String>,
}

impl PushData {
    async fn run(&self, operator: &Operator) -> i16 {
        match operator.copy("path/to/file", "path/to/file2").await {
            Ok(_) => 0,
            Err(err) => panic!("{:?}", err),
        }
    }
}
