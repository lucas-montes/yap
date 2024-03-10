use libsql::{params, Connection, Database};
use menva::get_env;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use crate::config::Config;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct File {
    remote: String,
    path: String,
    branch: String,
    timestamp: i64,
}

impl File {
    pub fn new(path: &PathBuf, branch: &str, timestamp: &i64) -> Self {
        Self {
            path: path.to_str().expect("unable to convert to str").to_string(),
            branch: branch.to_string(),
            timestamp: timestamp.to_owned(),
            ..Self::default()
        }
    }

    fn original_path(&self) -> String {
        PathBuf::from(&self.path)
            .canonicalize()
            .expect("unable to canonicalize")
            .to_str()
            .expect("unable to convert to str")
            .to_string()
    }

    pub fn history_path(&self, config: &Config) -> String {
        format!(
            "{}/{}/{}/{}",
            &config.history, &self.path, &self.branch, &self.timestamp
        )
    }

    pub async fn create_snapshot(&self, config: &Config) {
        Self::duplicate(&self.original_path(), &self.history_path(config));

        insert_snapshot(self, &config.logbooks()).await;
        insert_log(self, config).await;
    }

    fn duplicate(file: &str, local_path: &str) {
        let origin = std::fs::canonicalize(file).expect("Unable to canonicalize data origin");
        fs::create_dir_all(
            &PathBuf::from(local_path)
                .parent()
                .expect("unable to get duplciate local path parent"),
        )
        .expect("Couldn't create dirs");
        fs::copy(origin, &local_path).expect("Couldn't copy the file");
    }
}

pub async fn run() -> i16 {
    let _config = Config::new();
    0
}

pub fn get_or_create_local_db(path: &str) -> Connection {
    fs::create_dir_all(
        PathBuf::from(path)
            .parent()
            .expect("no parent for local db"),
    )
    .expect("unable to craete path to local db");
    let db = Database::open(path).expect("unable to open local db");
    db.connect().expect("Unable to connect to local db")
}

async fn insert_log(file: &File, config: &Config) {
    let logbook = get_user_logbook(config).await;
    logbook
        .execute(
            "CREATE TABLE IF NOT EXISTS events (file VARCHAR(150) NOT NULL, branch VARCHAR(150) NOT NULL, timestamp TIMESTAMP NOT NULL, event VARCHAR(10) NOT NULL);",
            (),
        )
        .await
        .expect("unable to create table events into user logbook");
    logbook
        .execute(
            "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
            params![
                file.path.clone(),
                file.branch.clone(),
                file.timestamp,
                "ADD"
            ],
        )
        .await
        .expect("error executing insert");
}

async fn insert_snapshot(file: &File, logbooks: &str) {
    let file_branch_db = get_or_create_local_db(&format!("{}/{}.db", logbooks, &file.path));

    file_branch_db
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} (timestamp TIMESTAMP NOT NULL);",
                &file.branch
            ),
            (),
        )
        .await
        .expect("unable to save movement into user logbook");
    // TODO: each time we'll need to check that the branch exists or not depending
    // on if we commit or add
    file_branch_db
        .execute(
            &format!("INSERT INTO {} (timestamp) VALUES (?1)", &file.branch),
            params![&file.timestamp],
        )
        .await
        .expect("unable to save movement into user logbook");
}

pub async fn get_user_logbook(config: &Config) -> Connection {
    let token = get_env("DB_TOKEN");
    let db = Database::open_with_remote_sync(config.local.to_str().unwrap(), &config.remote, token)
        .await
        .expect("unable to open to remote");
    db.connect().expect("unable to connect to remote db")
}
