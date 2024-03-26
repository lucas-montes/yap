use menva::get_env;
use opendal::{
    services::{Gcs, Koofr, Pcloud},
    Operator,
};
use serde::{Deserialize, Serialize};

use clap::ValueEnum;

use crate::data::{FileFacade, Remote};

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct RemoteConfig {
    #[serde(default)]
    storage: Storage,
    #[serde(default)]
    root: String,
    #[serde(default)]
    bucket: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    credentials: String,
    #[serde(default)]
    strategy: PushStrategy,
}

impl RemoteConfig {
    pub fn set_strategy(mut self, strategy: &PushStrategy) -> Self {
        self.strategy = strategy.to_owned();
        self
    }

    pub fn set_storage(mut self, storage: &Storage) -> Self {
        self.storage = storage.to_owned();
        self
    }

    fn password(&self) -> String {
        get_env(&self.password)
    }

    fn get_storage_operator(&self) -> Operator {
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
        builder.email(&self.username);
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
        builder.username(&self.username);
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
    #[default]
    Gcs,
    Koofr,
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

pub async fn push_file(file: &FileFacade) -> Remote {
    //TODO: according to the push strategy we need to get and loop all the versions
    let remote = file.remote();
    let operator = remote.get_storage_operator();
    let data = tokio::fs::read(file.original_path()).await.unwrap();
    operator
        .write(file.path().to_str().unwrap(), data)
        .await
        .unwrap();
    Remote::new(file.path().to_path_buf(), remote.strategy, remote.storage)
}

pub async fn pull_file(file: &FileFacade) {
    // TODO: when pushing and pulling from remote, how to know which files to get and send? keep
    // the strategy think? probably no. How should we save the files on the remote? as they are in
    // the history_path? probably.
    if file.path().exists() {
        println!("Already exists: {:?}", file.path())
    }
    let result = file
        .remote()
        .get_storage_operator()
        .read(file.path().to_str().unwrap())
        .await
        .unwrap();
    tokio::fs::create_dir_all(&file.path().parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&file.path(), &result).await.unwrap();
    println!("{:?} downloaded", file.path())
}
