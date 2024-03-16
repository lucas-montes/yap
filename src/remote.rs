use menva::get_env;
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
        Self::default().set_credentials()
    }

    pub fn set_strategy(mut self, strategy: &PushStrategy) -> Self {
        self.strategy = strategy.to_owned();
        self
    }

    pub fn set_storage(mut self, storage: &str) -> Self {
        self.storage = storage.to_owned();
        self
    }

    fn set_credentials(mut self) -> Self {
        self.credentials = get_env("GSC_CREDENTIALS");
        self
    }

    fn protocol(&self) -> Protocol {
        match self.storage.split_once("://") {
            Some((protocol, _)) => Protocol::from_str(protocol),

            None => panic!("What df is that protocol bro"),
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

#[derive(Debug)]
pub enum Protocol {
    Gs,
    Http,
}

impl Protocol {
    fn from_str(protocol: &str) -> Self {
        match protocol {
            "gs" | "Gs" => Protocol::Gs,
            "http" | "https" => Protocol::Http,
            _ => Protocol::Http,
        }
    }
}

pub async fn push_file(file: &FileFacade) {
    //TODO: according to the push strategy we need to get and loop all the versions
    let operator = get_operator(file);
    let result = operator
        .copy(
            file.original_path().to_str().unwrap(),
            file.path().to_str().unwrap(),
        )
        .await;
    println!("{:?}", result);
}

fn get_operator(file: &FileFacade) -> Operator {
    let remote = file.remote();
    match remote.protocol() {
        Protocol::Gs => create_gcs(
            &remote.storage,
            file.path().to_str().unwrap(),
            &remote.credentials,
        ),
        _ => todo!(),
    }
}

fn create_gcs(bucket: &str, file_path: &str, credentials: &str) -> Operator {
    let mut builder = Gcs::default();

    // set the storage bucket for OpenDAL
    builder.bucket(bucket);
    // set the working directory root for GCS
    // all operations will happen within it
    builder.root(file_path);
    // set the credential of service account.
    builder.credential_path(credentials);
    // set the predefined ACL for GCS
    builder.predefined_acl("publicRead");
    // set the default storage class for GCS
    builder.default_storage_class("STANDARD");

    match Operator::new(builder) {
        Ok(op) => op.finish(),
        Err(err) => panic!("{:?}", err),
    }
}
