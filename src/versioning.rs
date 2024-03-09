use hex::ToHex;
use libsql::{Connection, Database, Row, Rows};
use menva::get_env;
use meowhash::MeowHasher;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::MetadataExt;
use std::{fs, path::PathBuf};

use crate::config::Author;
use crate::config::Config;

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

struct Branch{}

struct FileTracked{
    id: u32,
    name: String,
    //TODO: maybe use a VecDeque
    branches: Vec<Branch>,
    versions: Vec<Commit>
}

struct Tracked{
files: Vec<FileTracked>

}

struct FileHistory {
    name: String,
    remote: String,
    history: History,
}

pub struct History {
    path: PathBuf,
    commits: Vec<Commit>,
}

impl History {
    pub fn from<T: AsRef<std::ffi::OsStr>>(p: &T) -> Self {
        let path = PathBuf::from(p);
        let commits = Self::get_commits(&path);
        Self {
            path: path,
            commits: commits,
        }
    }

    pub async fn new_current_history(&self) -> PathBuf {
        // TODO: find a better name
        let mut current_history = self.path.clone();
        let now = chrono::offset::Local::now().timestamp().to_string();
        current_history.push(now);
        std::fs::create_dir(&current_history).unwrap();
        current_history
    }

    fn get_commits(path: &PathBuf) -> Vec<Commit> {
        std::fs::read_dir(path)
            .expect("Unable to get the local history commits")
            .map(Commit::from)
            .collect()
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Commit {
    id: u32,
    created_at: String,
    updated_at: String,
    message: String,
    git_commit: String,
    timestamp: i64,
    file_from_id: u32,
    file_to_id: u32,
    diff_id: u32,
    author: Author,
}

impl Commit {
    pub fn from<T>(p: T) -> Self {
        todo!()
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct File {
    id: u32,
    hash: String,
    created_at: String,
    updated_at: String,
    remotes: String,
    path: String,
    size: u64,
}

impl File {
    pub fn new(path: &PathBuf) -> Self {
        let data = fs::read(path).expect("Unable to read the data file");
        let file_meta = path.metadata().expect("File has no metadata");
        let path = path.to_str().unwrap().to_owned();
        Self {
            hash: MeowHasher::hash(&data).into_bytes().encode_hex::<String>(),
            path: path,
            size: file_meta.size(),
            ..Self::default()
        }
    }

    pub fn duplicate(&self, mut history: PathBuf) {
        history.push(&self.path);
        let origin = std::fs::canonicalize(&self.path).expect("Unable to canonicalize data origin");
        fs::create_dir_all(&history.parent().expect("Data has no parent"))
            .expect("Couldn't create dirs");
        fs::copy(origin, &history).expect("Couldn't copy the file");
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
        let _result = conn.execute_batch(&stmts).await.expect("Cannot save to DB");
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
    println!("opening db");
    let db = Database::open_with_remote_sync(config.local.to_str().unwrap(), &config.remote, token)
        .await
        .unwrap();
    println!("db opened");
    let _conn = db.connect().unwrap();
    println!("synquing");
    let r = db.sync().await;
    println!("{r:?}");
}

struct RetentionPolicy {}
