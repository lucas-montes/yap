use core::panic;
use std::io::prelude::*;
use std::path::PathBuf;
use std::{fs::File, path::Path};

use clap::{Args, Subcommand};
use libsql::Database;
use serde::{Deserialize, Serialize};

use crate::{
    enums::ColorWhen,
    turso::{
        DatabasesPlatform, GroupsPlatform, LocationsPlatform, OrganizationsPlatform, TursoClient,
    },
};

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
    /// Creates the values in the configurations file if they dont exists. In case that some
    /// informations may be empty they will be filled if possible
    Init,
}

impl ConfigCommands {
    pub async fn handle_commands(&self) -> i16 {
        match self {
            ConfigCommands::Add(args) => args.add().await,
            ConfigCommands::Set(args) => args.set().await,
            ConfigCommands::Get(args) => args.get().await,
            ConfigCommands::Remove(args) => args.remove().await,
            ConfigCommands::Init => Config::init().await,
        };
        0
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Author {
    name: String,
    email: String,
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
    // TODO: maybe the attributes below should be able to be passed by stdin
    #[clap(skip)]
    #[serde(default)]
    pub history: String,
    #[clap(skip)]
    #[serde(default)]
    pub author: Author,
}

impl Config {
    fn check_for_root() {
        match Path::new(".yap").exists() {
            true => (),
            false => panic!("oupsi daisy no you are not in the root buddy"),
        }
    }

    pub fn new() -> Config {
        // lets enfore being in the root of the project to launch the cli for the moment
        // TODO: do something better to check and be in root. Maybe something like git
        Config::check_for_root();
        toml_to_struct(".yap/.config")
    }

    async fn set_default_location(&mut self, client: &TursoClient<LocationsPlatform>) -> &mut Self {
        if self.location.is_empty() {
            match client.closest_region().await {
                Ok(l) => self.location = l.client,
                Err(err) => panic!("{err:?}"),
            };
        }
        self
    }

    async fn set_default_organization(
        &mut self,
        client: &TursoClient<OrganizationsPlatform>,
    ) -> &mut Self {
        if self.organization.is_empty() {
            self.organization = client
                .list()
                .await
                .unwrap()
                .iter()
                .find(|o| o.name == "personal")
                .unwrap()
                .slug
                .clone();
        }
        self
    }

    async fn set_default_group(&mut self, client: &TursoClient<GroupsPlatform>) -> &mut Self {
        // if we delete our only database related to a group it seems that it deletes the group too
        if self.group.is_empty() | self.remote.is_empty() {
            let default_name = "yap-default";
            let groups = client.list(&self.organization).await.unwrap().groups;

            let default_group = groups.iter().find(|g| g.name == default_name);
            if groups.is_empty() | default_group.is_none() {
                let new_group = client
                    .create(&self.organization, default_name, &self.location)
                    .await
                    .unwrap();
                self.group = new_group.group.name.clone();
            } else if default_group.is_some() {
                let default_group = default_group.unwrap();
                self.group = default_group.name.clone();
            };
        }
        self
    }

    async fn set_default_local_db(&mut self) -> &mut Self {
        if !self.local.is_file() {
            let default_name = ".yap/local.db";
            let query = match tokio::fs::read_to_string("migrations.sql").await {
                Ok(sql) => sql,
                Err(err) => {
                    panic!("{:?}", err);
                }
            };
            let db = Database::open(default_name).unwrap();
            let conn = db.connect().unwrap();
            conn.execute(&query, ()).await.unwrap();
            self.local = default_name.into();
        }
        self
    }

    async fn set_default_remote_db(
        &mut self,
        client: &TursoClient<DatabasesPlatform>,
    ) -> &mut Self {
        if self.remote.is_empty() {
            let default_name = "yap-default";
            let databases = client.list(&self.organization).await.unwrap();
            let default_db = databases.databases.iter().find(|g| g.name == default_name);
            let mut remote = String::from("libsql://");
            if default_db.is_some() {
                remote.push_str(&default_db.unwrap().hostname.to_owned());
            } else {
                let new_db = client
                    .create(&self.organization, default_name, &self.group)
                    .await
                    .unwrap();
                remote.push_str(&new_db.database.hostname.to_owned());
            }
            self.remote = remote;
        }
        self
    }

    async fn set_default_history(&mut self) -> &mut Self{
        let history_path = Path::new(".yap/history");
        if !history_path.exists() {
            std::fs::create_dir(history_path).unwrap()
        }
        self
    }

    pub async fn new_current_history(&self) -> PathBuf {
        let mut current_history = PathBuf::from(self.history.clone());
        let now = chrono::offset::Local::now().timestamp().to_string();
        current_history.push(now);
        std::fs::create_dir_all(&current_history).unwrap();
        current_history
    }

    async fn init() {
        let mut file_config = Config::new();
        let client = TursoClient::new().locations();

        file_config.set_default_location(&client).await;
        file_config.set_default_history().await;

        let client = client.organizations();
        file_config.set_default_organization(&client).await;

        let client = client.groups();
        file_config.set_default_group(&client).await;

        file_config.set_default_local_db().await;

        let client = client.databases();
        file_config.set_default_remote_db(&client).await;

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
    let mut file = File::create(path).expect("Unable to open file");
    let _ = file.write_all(data.as_bytes());
}
