use crate::config::Config;
use libsql::{params, Connection, Database};
use menva::get_env;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    env, fs,
    path::{Path, PathBuf},
};
use turso::RowsIter;

use crate::enums::Events;

use super::comparaison::{
    compare_custom, compare_hash, compare_similarity, compare_smart, Comparaison,
    ComparaisonTechnique,
};

//  TODO: pas more things as ref

pub struct FileFacadeFactory {
    branch: String,
    config: Config,
    timestamp: i64,
    stack: VecDeque<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct FileConfig {
    #[serde(default)]
    remote: String,
    #[serde(default)]
    comparaison: Comparaison,
}

impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str, config: &Config) -> Self {
        Self {
            branch: branch.to_owned(),
            config: config.to_owned(),
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
                p.read_dir()
                    .unwrap()
                    .for_each(|p| self.stack.push_back(p.unwrap().path()));
            }
            Some(FileFacade::new(
                &p,
                &self.branch,
                self.timestamp,
                &self.config,
            ))
        })
    }
}

pub struct FileFacade {
    changed: bool,
    path: PathBuf,
    history: String,
    branch: String,
    timestamp: i64,
    //TODO: maybe instead of multiple connections we could create only one and save everything in
    //batches
    conn: Connection,
}

impl FileFacade {
    pub fn new(path: &PathBuf, branch: &str, timestamp: i64, config: &Config) -> Self {
        Self {
            changed: true,
            history: config.history_dir(),
            path: path.to_owned(),
            branch: branch.to_string(),
            timestamp,
            conn: Self::get_or_create_local_db(&config.logbooks_dir(), &path),
        }
    }

    pub fn previous_version(&self) -> PathBuf {
        //TODO: for now just look in the history dir, later on check the dabbatase and find the
        //previous file even if remote
        let mut files = self
            .history_dir()
            .read_dir()
            .unwrap()
            .map(|f| {
                f.unwrap()
                    .file_name()
                    .into_string()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
            })
            .collect::<Vec<i64>>();
        files.sort();
        //TODO: poping the last elemnt will do the trick for now. find a better way to find the
        //previous
        let prev_version = files.pop().unwrap().to_string();
        self.history_dir().join(Path::new(&prev_version))
    }

    pub fn compare(
        &mut self,
        technique: &ComparaisonTechnique,
        script: &Option<PathBuf>,
    ) -> &mut Self {
        self.changed = !match technique {
            ComparaisonTechnique::Hash => compare_hash(self),
            ComparaisonTechnique::Custom => compare_custom(
                self,
                script.as_ref().expect(
                    "You must provide a script if you select the custom comparaison technique",
                ),
            ),
            ComparaisonTechnique::Similarity => compare_similarity(self),
            ComparaisonTechnique::Smart => compare_smart(self),
        };
        self
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }

    fn get_or_create_local_db(logbooks: &PathBuf, path: &PathBuf) -> Connection {
        //TODO: will path always be relative?
        let mut db_path = path.to_str().unwrap().to_string();
        db_path.push_str(".db");
        let final_path = logbooks.clone().join(db_path);
        fs::create_dir_all(final_path.parent().expect("no parent for local db"))
            .expect("unable to craete path to local db");
        let db = Database::open(
            &final_path
                .to_str()
                .expect("unable to conver to str")
                .to_string(),
        )
        .expect("unable to open local db");
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
            .expect("unable to save movement into project logbook");
        self
    }

    pub fn original_path(&self) -> PathBuf {
        //TODO: will path always be relative?
        env::current_dir().unwrap().join(&self.path)
    }

    pub fn history_dir(&self) -> PathBuf {
        env::current_dir()
            .unwrap()
            .join(&self.history)
            .join(&self.path)
            .join(&self.branch)
    }

    pub fn history_path(&self) -> PathBuf {
        //TODO: will path always be relative?
        self.history_dir()
            .join(Path::new(&self.timestamp.to_string()))
    }

    pub fn duplicate(&self) -> &Self {
        let file = self.original_path();
        let local_path = self.history_path();
        // TODO: make this async? can be done with tokio?
        fs::create_dir_all(
            local_path
                .parent()
                .expect("unable to get duplciate local path parent"),
        )
        .expect("Couldn't create dirs");
        fs::create_dir_all(&local_path.parent().unwrap()).unwrap();
        match fs::copy(&file, &local_path) {
            Ok(_) => self,
            Err(err) => panic!("{}", err),
        }
    }
}

pub struct Logbook {
    db: Database,
}

impl Logbook {
    pub fn conn(&self) -> Connection {
        self.db.connect().expect("cant connect to db")
    }

    pub fn local(path: &str) -> Self {
        let db = Database::open(path).expect("unable to open local");
        Self::new(db)
    }

    pub async fn remote(local_db: &str, remote_db: &str) -> Self {
        let token = get_env("DB_TOKEN");
        let db = Database::open_with_remote_sync(local_db, remote_db, token)
            .await
            .expect("unable to open to remote");
        Self::new(db)
    }

    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn file_is_tracked(&self, file: &FileFacade, event: &Events) -> bool {
        let mut result = self.conn()
            .query(
                "SELECT EXISTS(SELECT 1 FROM events WHERE file=?1 AND branch=?2 AND event=?3 LIMIT 1);",
                (file.path.to_str().expect("unable to convert to str"), file.branch.clone(), event.to_string()),
            )
            .await
            .expect("error executing insert");
        0 != RowsIter::new(&mut result)
            .next()
            .expect("iterator empty")
            .get::<u32>(0)
            .expect("couldnt get the value")
    }

    pub async fn insert_log(&self, file: &FileFacade, event: &Events) {
        self.conn()
            .execute(
                "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
                params![
                    file.path.to_str().expect("unable to convert to str"),
                    file.branch.clone(),
                    file.timestamp,
                    event.to_string()
                ],
            )
            .await
            .expect("error executing insert");
    }
}
pub async fn run() -> i16 {
    let _config = Config::new();
    0
}
