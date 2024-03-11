use crate::config::Config;
use libsql::{params, Connection, Database};
use std::{collections::VecDeque, fmt, fs, path::PathBuf};

pub struct FileFacadeFactory {
    branch: String,
    logbooks: String,
    timestamp: i64,
    stack: VecDeque<PathBuf>,
}

impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str, logbooks: &str) -> Self {
        Self {
            branch: branch.to_owned(),
            logbooks: logbooks.to_owned(),
            timestamp: chrono::offset::Local::now().timestamp(),
            stack: VecDeque::from(paths),
        }
    }
}

impl Iterator for FileFacadeFactory {
    type Item = FileFacade;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop_front().and_then(|p| {
            if p.is_dir() {
                fs::read_dir(&p)
                    .unwrap()
                    .for_each(|p| self.stack.push_back(p.unwrap().path()));
            }
            Some(FileFacade::new(
                p.to_str().expect("unable to convert to str"),
                &self.branch,
                self.timestamp,
                &self.logbooks,
            ))
        })
    }
}

pub struct FileFacade {
    remote: String,
    path: String,
    branch: String,
    timestamp: i64,
    conn: Connection,
}

impl FileFacade {
    pub fn new(path: &str, branch: &str, timestamp: i64, logbooks: &str) -> Self {
        Self {
            path: path.to_string(),
            branch: branch.to_string(),
            timestamp,
            conn: Self::get_or_create_local_db(logbooks, path),
            remote: String::default(),
        }
    }

    fn get_or_create_local_db(logbooks: &str, path: &str) -> Connection {
        fs::create_dir_all(
            PathBuf::from(format!("{}/{}.db", logbooks, path))
                .parent()
                .expect("no parent for local db"),
        )
        .expect("unable to craete path to local db");
        let db = Database::open(path).expect("unable to open local db");
        db.connect().expect("Unable to connect to local db")
    }

    pub async fn insert_snapshot(&self) -> &Self {
        self.conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (timestamp TIMESTAMP NOT NULL);",
                    &self.branch
                ),
                (),
            )
            .await
            .expect("unable to save movement into user logbook");

        self.conn
            .execute(
                &format!("INSERT INTO {} (timestamp) VALUES (?1)", &self.branch),
                params![&self.timestamp],
            )
            .await
            .expect("unable to save movement into user logbook");
        self
    }

    fn original_path(&self) -> String {
        PathBuf::from(&self.path)
            .canonicalize()
            .expect("unable to canonicalize")
            .to_str()
            .expect("unable to convert to str")
            .to_string()
    }

    fn history_path(&self, history: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            history, &self.path, &self.branch, &self.timestamp
        )
    }

    pub fn duplicate(&self, history: &str) -> &Self {
        let file = self.original_path();
        let local_path = self.history_path(history);
        // TODO: make this async?
        let origin = std::fs::canonicalize(file).expect("Unable to canonicalize data origin");
        fs::create_dir_all(
            &PathBuf::from(local_path)
                .parent()
                .expect("unable to get duplciate local path parent"),
        )
        .expect("Couldn't create dirs");
        fs::copy(origin, &local_path).expect("Couldn't copy the file");
        self
    }
}

pub struct Logbook {
    conn: Connection,
}

impl Logbook {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub async fn create_table_events(&self) {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS events (file VARCHAR(150) NOT NULL, branch VARCHAR(150) NOT NULL, timestamp TIMESTAMP NOT NULL, event VARCHAR(10) NOT NULL);",
                (),
            )
            .await
            .expect("unable to create table events into user logbook");
    }

    pub async fn file_is_tracked(&self, file: &FileFacade) -> bool {
        0 == self.conn
            .execute(
                "SELECT EXISTS(SELECT 1 FROM events WHERE file=?1 AND branch=?2 AND event=?3 LIMIT 1);",
                params![file.path.clone(), file.branch.clone(), "ADD"],
            )
            .await
            .expect("error executing insert")
    }

    pub async fn insert_log(&self, file: &FileFacade, event: &Events) {
        // TODO: move into the init function the creation of the table
        self.create_table_events().await;
        self.conn
            .execute(
                "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
                params![
                    file.path.clone(),
                    file.branch.clone(),
                    file.timestamp,
                    event.to_string()
                ],
            )
            .await
            .expect("error executing insert");
    }
}

#[derive(Debug)]
pub enum Events {
    Add,
    Commit,
    Push,
}

impl fmt::Display for Events {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
pub async fn run() -> i16 {
    let _config = Config::new();
    0
}
