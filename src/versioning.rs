use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;
use std::{fs, path::PathBuf};

use hex::ToHex;
use libsql::{de, params, Connection, Database, Row, Rows};
use menva::get_env;
use meowhash::MeowHasher;
use serde::{Deserialize, Serialize};

use crate::{config::Config, turso::TursoClient};

struct RowsIter<'a> {
    rows: &'a mut Rows,
}

impl<'a> RowsIter<'a> {
    fn new(rows: &'a mut Rows) -> Self {
        Self { rows }
    }
}

impl<'a> Iterator for RowsIter<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match self.rows.next() {
            Ok(row) => row,
            Err(err) => panic!("some error in the iterator getting the next row: {err:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct File {
    id: u32,
    hash: String,
    created_at: String,
    updated_at: String,
    remotes: String,
    next: Option<u32>,
    previous: Option<u32>,
    path: String,
    size: u64,
}

impl File {
    pub fn new(path: &PathBuf) -> Self {
        let data = fs::read(path).unwrap();
        let file_meta = path.metadata().unwrap();
        Self {
            hash: MeowHasher::hash(&data).into_bytes().encode_hex::<String>(),
            path: path.to_str().unwrap().to_owned(),
            size: file_meta.size(),
            ..Self::default()
        }
    }

    pub fn duplicate(&self, mut history: PathBuf) {
        history.push(&self.path);
        fs::copy(&self.path, &history).unwrap();
    }

    fn to_insert_query(&self) -> String {
        format!(
            "INSERT INTO files (hash, path, size) VALUES ('{}', '{}', {})",
            &self.hash, &self.path, &self.size
        )
    }

    pub async fn add_many(conn: &Connection, files: &Vec<File>) {
        let mut stmts = files
            .iter()
            .map(|f| f.to_insert_query())
            .collect::<Vec<String>>()
            .join(";");
        stmts.push_str("end;");
        let _result = conn.execute_batch(&stmts).await.unwrap();
    }
}

pub async fn run() -> i16 {
    let config = Config::new();
    sync_remote(&config).await;
    0
}

pub async fn connect_db() -> Connection {
    let db = Database::open(".yap/local.db").unwrap();
    db.connect().unwrap()
}

async fn sync_remote(config: &Config) {
    let token = get_env("DB_TOKEN");
    let client = TursoClient::new().databases();
    println!("opening db");
    let db = Database::open_with_remote_sync(config.local.to_str().unwrap(), &config.remote, token)
        .await
        .unwrap();
    println!("db opened");
    let conn = db.connect().unwrap();
    println!("synquing");
    let r = db.sync().await;
    println!("{r:?}");
}
