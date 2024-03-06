use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;
use std::{fs, path::PathBuf};

use hex::ToHex;
use libsql::{de, params, Connection, Database, Row, Rows};
use menva::get_env;
use meowhash::MeowHasher;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    turso::{DatabasesPlatform, GroupsPlatform, RetrievedDatabase, TursoClient},
};

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

fn compress_file(input: &[u8], level: i32) {
    let compressed = zstd::encode_all(input, level).unwrap();
    let decoded: Vec<u8> = zstd::decode_all(compressed.as_slice()).unwrap();
    let decoded_text = std::str::from_utf8(&decoded).unwrap();
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
    local: String,
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

    pub fn remotes(&self) -> Vec<&str> {
        self.remotes.split(",").collect()
    }

    fn to_insert_query(&self) -> String {
        format!(
            "INSERT INTO files (hash, path, size, local) VALUES ('{}', '{}', {}, '{}')",
            &self.hash, &self.path, &self.size, &self.local,
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

    async fn create(conn: &Connection, hash: &str, path: &str) -> Result<(), libsql::Error> {
        let r = conn
            .execute(
                "INSERT INTO files (hash, path) VALUES (?1, ?2)",
                params![hash, path],
            )
            .await?;
        println!("result for {path} {r} in create");
        Ok(())
    }

    async fn get_by_id(conn: &Connection, id: u32) -> Option<File> {
        let mut result = conn
            .query("SELECT * FROM files WHERE id = ?1", params![id])
            .await
            .unwrap();

        result
            .next()
            .unwrap()
            .and_then(|r| de::from_row(&r).expect("no row found in get by id"))
    }

    async fn update(conn: &Connection, id: u32, new_path: &str) -> Result<(), libsql::Error> {
        conn.execute(
            "UPDATE files SET path = ?1 WHERE id = ?2",
            params![new_path, id],
        )
        .await?;
        Ok(())
    }

    async fn delete_by_id(conn: &Connection, id: u32) -> Result<(), libsql::Error> {
        conn.execute("DELETE FROM Files WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }
    async fn get_all(conn: &Connection) -> Result<Vec<File>, libsql::Error> {
        let mut results = conn.query("SELECT * FROM files", ()).await?;
        Ok(RowsIter::new(&mut results)
            .map(|r| de::from_row::<File>(&r).unwrap())
            .collect::<Vec<File>>())
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
