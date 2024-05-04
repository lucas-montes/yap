use core::panic;
use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::{
    enums::ColorWhen,
    utils::{struct_to_toml, toml_to_struct},
};
use clap::{Args, Subcommand, ValueEnum};
use libsql::Builder;
use menva::get_env;
use opendal::{
    services::{Gcs, Koofr, Pcloud},
    Operator,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use turso::{
    DatabasesPlatform, GroupsPlatform, LocationsPlatform, OrganizationsPlatform,
    TursoClient,
};
//TODO: somewhere add the --env to pass the file for the envs
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
/// TODO: for all configs make strings options if it can reduce memory size

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Author {
    pk: Option<u32>,
    name: String,
    email: String,
}

impl Author {
    pub fn pk(&self) -> String {
        format!("{} <{}<>", &self.name, &self.email)
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

#[derive(Debug, Args, Deserialize, Default, Serialize, PartialEq, Clone)]
pub struct TursoConfig {
    #[arg(short, long, required = false)]
    #[serde(default)]
    organization: String,
    #[arg(short, long, required = false)]
    #[serde(default)]
    group: String,
    #[arg(long, required = false)]
    #[serde(default)]
    location: String,
    #[arg(short, long, required = false)]
    #[serde(default)]
    remote_db: String,
}

impl TursoConfig {
    async fn set_default_location(
        &mut self,
        client: &TursoClient<LocationsPlatform>,
    ) -> &mut Self {
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

    async fn set_default_group(
        &mut self,
        client: &TursoClient<GroupsPlatform>,
    ) -> &mut Self {
        // if we delete our only database related to a group it seems that it deletes the group too
        if self.group.is_empty() | self.remote_db.is_empty() {
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

    async fn set_default_remote_db(
        &mut self,
        client: &TursoClient<DatabasesPlatform>,
    ) -> &mut Self {
        if self.remote_db.is_empty() {
            let default_name = "yap-master";
            let databases = client.list(&self.organization).await.unwrap();
            let default_db = databases.databases.iter().find(|g| g.name == default_name);
            let mut remote = String::from("libsql://");
            if let Some(db) = default_db {
                remote.push_str(&db.hostname.to_owned());
            } else {
                let new_db = client
                    .create(&self.organization, default_name, &self.group)
                    .await
                    .unwrap();
                remote.push_str(&new_db.database.hostname.to_owned());
            }
            self.remote_db = remote;
        }
        self
    }
}

#[derive(Debug, Args, Deserialize, Default, Serialize, PartialEq, Clone)]
pub struct Config {
    // Project or master config for the database movements to keep history of commits
    #[arg(long, required = false)]
    #[serde(default)]
    local_db: String,
    // Files history and dbs for commits history
    #[arg(long, required = false)]
    #[serde(default)]
    logbooks_dir: String,
    #[arg(long, required = false)]
    #[serde(default)]
    history_dir: String,
    #[clap(skip)]
    #[serde(default)]
    distributed_logbook: Option<TursoConfig>,
    #[clap(skip)]
    #[serde(default)]
    remote: RemoteConfig,
    #[clap(skip)]
    #[serde(default)]
    author: Author,
}

//TODO: make this file smaller so more settings are saved in the databse
impl Config {
    async fn init() {
        let mut file_config = Config::new();

        file_config.set_default_history_dir().await;
        file_config.set_default_logbooks_dir().await;
        file_config.set_default_local_db().await;

        if let Some(logbook_cfg) = file_config.distributed_logbook.as_mut() {
            let client = TursoClient::new().locations();

            logbook_cfg.set_default_location(&client).await;
            let client = client.organizations();
            logbook_cfg.set_default_organization(&client).await;

            let client = client.groups();
            logbook_cfg.set_default_group(&client).await;

            let client = client.databases();
            logbook_cfg.set_default_remote_db(&client).await;
        }
        struct_to_toml(&file_config, ".yap/.config");
    }

    //fn check_for_root() {
    //    // TODO: we are testing to track single files.once finished change it
    //    // we use the general .yap to track files whereever we are
    //    match Path::new(".yap").exists() {
    //        true => (),
    //        false => panic!("oupsi daisy no you are not in the root buddy"),
    //    }
    //}

    pub fn author(&self) -> Author {
        self.author.clone()
    }

    pub fn remote_storage(&self) -> RemoteConfig {
        //TODO: do wee need to clone?
        self.remote.clone()
    }

    pub fn root(&self, path: &str) -> String {
        //TOOD: use a path
        format!(".yap/{path}")
    }

    pub fn local_db(&self) -> String {
        self.root(&self.local_db)
    }
    pub fn logbooks_dir(&self) -> PathBuf {
        PathBuf::from(self.root(&self.logbooks_dir))
    }
    pub fn history_dir(&self) -> String {
        self.root(&self.history_dir)
    }

    pub fn new() -> Config {
        // TODO: do something better to check and be in root. Maybe something like git
        //Config::check_for_root();
        toml_to_struct(".yap/.config")
    }

    async fn set_default_local_db(&mut self) -> &mut Self {
        if self.local_db.is_empty() {
            let local_default = "logbook.db";
            self.local_db = local_default.to_string();
        }
        let db = Builder::new_local(self.local_db())
            .build()
            .await
            .expect("unable to open to local");
        let conn = db.connect().expect("unable to connect to local db");
        let query = match tokio::fs::read_to_string("schemas/master_logbook.sql").await {
            Ok(sql) => sql,
            Err(err) => {
                panic!("{err}");
            },
        };
        conn.execute_batch(&query)
            .await
            .expect("unable to create tables into file logbook");
        self
    }

    async fn set_default_history_dir(&mut self) -> &mut Self {
        if self.history_dir.is_empty() {
            self.history_dir = "history".to_string();
        }
        let history_dir = self.history_dir();
        let history_path = Path::new(&history_dir);
        if !history_path.exists() {
            fs::create_dir_all(history_path)
                .await
                .expect("cant create history dir");
        }
        self
    }

    async fn set_default_logbooks_dir(&mut self) -> &mut Self {
        if self.logbooks_dir.is_empty() {
            self.logbooks_dir = "logbooks".to_string();
        }
        let logbooks_dir = self.logbooks_dir();
        let logbooks_path = Path::new(&logbooks_dir);
        if !logbooks_path.exists() {
            fs::create_dir_all(logbooks_path)
                .await
                .expect("cant create logbooks dir");
        }
        self
    }

    async fn add(&self) {}

    async fn set(&self) {}

    async fn get(&self) {}

    async fn remove(&self) {}
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct RemoteConfig {
    #[serde(default)]
    pub storage: Storage,
    #[serde(default)]
    root: String,
    #[serde(default)]
    bucket: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    credentials: String,
    #[serde(default)]
    pub strategy: PushStrategy,
}

impl RemoteConfig {
    fn password(&self) -> String {
        get_env(self.credentials.as_str())
    }

    pub fn get_storage_operator(&self) -> Operator {
        match self.storage {
            Storage::Gcs => self.create_gcs(),
            Storage::Koofr => self.create_koofr(),
            Storage::Pcloud => self.create_pcloud(),
        }
    }

    fn create_koofr(&self) -> Operator {
        let mut builder = Koofr::default();
        builder.root(&self.root);
        builder.endpoint("https://api.koofr.net");
        builder.email(
            self.username
                .as_ref()
                .expect("You need to set your email address for the username field"),
        );
        builder.password(&self.password());
        match Operator::new(builder) {
            Ok(op) => op.finish(),
            Err(err) => panic!("{:?}", err),
        }
    }

    fn create_pcloud(&self) -> Operator {
        let mut builder = Pcloud::default();
        builder.root(&self.root);
        builder.endpoint("https://eapi.pcloud.com");
        builder.username(
            self.username
                .as_ref()
                .expect("You has to set the username value"),
        );
        builder.password(&self.password());

        match Operator::new(builder) {
            Ok(op) => op.finish(),
            Err(err) => panic!("{:?}", err),
        }
    }

    fn create_gcs(&self) -> Operator {
        let mut builder = Gcs::default();
        builder.bucket("yap-default");
        builder.root(&self.root);
        builder.credential_path(&self.credentials);
        builder.predefined_acl("publicRead");
        builder.default_storage_class("STANDARD");

        match Operator::new(builder) {
            Ok(op) => op.finish(),
            Err(err) => panic!("{:?}", err),
        }
    }
}

#[derive(ValueEnum, Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
pub enum PushStrategy {
    All,
    Last,
    #[default]
    Smart,
}

impl std::fmt::Display for PushStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(ValueEnum, Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub enum Storage {
    Gcs,
    Koofr,
    #[default]
    Pcloud,
}

impl std::fmt::Display for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
