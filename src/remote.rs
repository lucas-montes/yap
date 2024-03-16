use opendal::{services::Gcs, Operator};
use serde::{Deserialize, Serialize};

use clap::ValueEnum;

use crate::data::FileFacade;

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Remote {
    #[serde(default)]
    storage: String,
    #[serde(default)]
    credentials: String,
    #[serde(default)]
    strategy: PushStrategy,
}

impl Remote {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_strategy(mut self, strategy: &PushStrategy) -> Self {
        self.strategy = strategy.to_owned();
        self
    }
    pub fn set_storage(mut self, storage: &str) -> Self {
        self.storage = storage.to_owned();
        self
    }
    fn protocol(&self) -> Protocol {
        todo!()
    }
}

#[derive(ValueEnum, Debug, Default, Clone, Deserialize, PartialEq, Serialize)]
pub enum PushStrategy {
    All,
    Last,
    #[default]
    Smart,
}

#[derive(Debug)]
pub enum Protocol {
    Ssh(String),
    Ftp(String),
    Gcs(String),
    Http(String),
}

pub fn push_file(file: &FileFacade) {
    match file.remote() {
        //"md" => compare_similarity(&file),
        _ => todo!(),
    }
}

fn create_operator() -> Operator {
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
