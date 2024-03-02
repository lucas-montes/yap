use core::panic;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use libsql::Database;
use menva::get_env;
use serde::{Deserialize, Serialize};
use toml::value::Table;

use crate::{
    enums::ColorWhen,
    turso::{DatabasesPlatform, GroupsPlatform, RetrievedDatabase, TursoClient},
};

#[derive(Debug, Deserialize)]
struct Core {
    remote: String,
}

#[derive(Debug, Deserialize)]
struct RemoteConfig {
    url: String,
}

#[derive(Debug, Deserialize)]
struct DvcConfig {
    core: Core,

    #[serde(flatten)]
    remotes: Table,
}

impl DvcConfig {
    pub fn new() -> Self {
        toml_to_struct(".dvc")
    }
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
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
pub enum ConfigCommands {
    /// Add configurations
    Add(Config),
    /// Add or override configurations
    Set(Config),
    /// Get Read configurations
    Get(Config),
    /// Remove configurations
    Remove(Config),
    /// Validate your configurations
    Validate,
    /// Creates the values in the configurations file if they dont exists. In case that some
    /// informations may be empty they will be filled if possible
    Apply,
}

impl ConfigCommands {
    pub async fn handle_commands(&self) -> i16 {
        match self {
            ConfigCommands::Add(args) => args.add().await,
            ConfigCommands::Set(args) => args.set().await,
            ConfigCommands::Get(args) => args.get().await,
            ConfigCommands::Remove(args) => args.remove().await,
            ConfigCommands::Validate => Config::validate().await,
            ConfigCommands::Apply => Config::apply().await,
        };
        0
    }
}

#[derive(Debug, Args, Deserialize, Default, Serialize, PartialEq, Clone)]
pub struct Config {
    //Turso config for the databases
    #[arg(short, long, required = false)]
    #[serde(default)]
    pub organization: String,
    #[arg(short, long, required = false)]
    #[serde(default)]
    pub local: PathBuf,
    #[arg(short, long, required = false)]
    #[serde(default)]
    pub remote: String,
    #[arg(short, long, required = false)]
    #[serde(default)]
    pub group: String,
    #[arg(short, long, required = false)]
    #[serde(default)]
    pub location: String,
}

impl Config {
    pub fn new() -> Self {
        toml_to_struct(".yap/.config")
    }

    async fn validate() {}

    async fn apply() {
        let mut file_config = Config::new();
        let client = TursoClient::new().locations();

        // Set closet location
        if file_config.location.is_empty() {
            match client.closet_region().await {
                Ok(l) => file_config.location = l.client,
                Err(err) => panic!("{err:?}"),
            };
        }

        // Set the organization if needed
        let client = client.organizations();
        if file_config.organization.is_empty() {
            file_config.organization = client
                .list()
                .await
                .unwrap()
                .iter()
                .find(|o| o.name == "default")
                .unwrap()
                .slug
                .clone();
        }

        // Create group if needed
        let client = client.groups();
        if file_config.group.is_empty() {
            if client
                .list(&file_config.organization)
                .await
                .unwrap()
                .groups
                .is_empty()
            {
                let default_name = "yap-default";
                let new_group = client
                    .create(
                        &file_config.organization,
                        default_name,
                        &file_config.location,
                    )
                    .await
                    .unwrap();
                file_config.group = new_group.group.name.clone();
            };
        }

        // Create local database if needed
        if !file_config.local.is_file() {
            let default_name = ".yap/yap-default.db";
            let db = Database::open(default_name).unwrap();
            let conn = db.connect().unwrap();
            //TODO: apply migrations
        }

        // Create remote database if needed
        let client = client.databases();
        if file_config.remote.is_empty() {
            let default_name = "yap-default";
            let new_db = client
                .create(&file_config.organization, default_name, &file_config.group)
                .await
                .unwrap();
            file_config.remote = new_db.database.hostname.to_owned();
        }

        struct_to_toml(&file_config, ".yap/.config");
    }

    async fn add(&self) {}

    async fn set(&self) {}

    async fn get(&self) {}

    async fn remove(&self) {}
}

fn toml_to_struct<T: for<'a> Deserialize<'a>>(path: &str) -> T {
    let mut file = File::open(path).expect("Unable to open file");
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)
        .expect("Unable to read file");
    toml::from_str(&toml_string).expect("Unable to parse TOML")
}

fn struct_to_toml<T: Serialize>(instance: &T, path: &str) {
    let data = toml::to_string_pretty(instance).expect("Unable to parse TOML");
    let mut file = File::open(path).expect("Unable to open file");
    file.write_all(data.as_bytes());
}
