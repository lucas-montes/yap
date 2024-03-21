use std::fs;

use menva::get_env;
use opendal::{
    services::{Gcs, Koofr, Pcloud},
    Operator,
};
use serde::{Deserialize, Serialize};

use clap::ValueEnum;

use crate::data::FileFacade;

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Remote {
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

impl Remote {
    pub fn new() -> Self {
        Self::default().set_credentials()
    }

    pub fn set_strategy(mut self, strategy: &PushStrategy) -> Self {
        self.strategy = strategy.to_owned();
        self
    }

    pub fn set_storage(mut self, storage: &Storage) -> Self {
        self.storage = storage.to_owned();
        self
    }

    fn set_credentials(mut self) -> Self {
        self.credentials = get_env("GSC_CREDENTIALS");
        self
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
        builder.endpoint("https://api.koofr.net/");
        builder.email(&self.username);
        builder.password(&self.password);
        match Operator::new(builder) {
            Ok(op) => op.finish(),
            Err(err) => panic!("{:?}", err),
        }
    }

    fn create_pcloud(&self) -> Operator {
        let mut builder = Pcloud::default();
        builder.root(&self.root);
        builder.endpoint("[https](https://eapi.pcloud.com)");
        builder.username(&self.username);
        builder.password(&self.password);

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

#[derive(ValueEnum, Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub enum Storage {
    #[default]
    Gcs,
    Koofr,
    Pcloud,
}

pub async fn push_file(file: &FileFacade) {
    //TODO: according to the push strategy we need to get and loop all the versions
    let remote = file.remote();
    let operator = remote.get_storage_operator();
    let data = tokio::fs::read(file.original_path()).await.unwrap();
    let result = operator.write(file.path().to_str().unwrap(), data).await;
}

pub async fn pull_file(file: &FileFacade) {
    let remote = file.remote();
    let operator = remote.get_storage_operator();
    let result = operator.read(file.path().to_str().unwrap()).await.unwrap();
    tokio::fs::create_dir_all(&file.history_path().parent().unwrap()).await.unwrap();
    tokio::fs::write(&file.history_path(), &result).await.unwrap();
}
