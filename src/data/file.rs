use libsql::{params, Connection, Database};
use menva::get_env;
use std::{
    collections::VecDeque,
    env, fs,
    path::{Path, PathBuf},
};
use turso::RowsIter;

use crate::{
    config::Config,
    remote::{push_file, Remote},
};
use crate::{enums::Events, remote::PushStrategy};

use super::comparaison::{
    compare_custom, compare_hash, compare_similarity, compare_smart, Comparaison,
    ComparaisonTechnique,
};

//  TODO: pass more things as ref
#[derive(Debug, Default)]
pub struct FileFacadeFactory {
    branch: String,
    history: String,
    timestamp: i64,
    remote: Option<Remote>,
    comparaison: Option<Comparaison>,
    stack: VecDeque<PathBuf>,
}

impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str, history: &str) -> Self {
        Self {
            branch: branch.to_owned(),
            history: history.to_owned(),
            timestamp: chrono::offset::Local::now().timestamp(),
            stack: VecDeque::from(paths),
            ..Self::default()
        }
    }

    pub fn set_remote(
        mut self,
        config: &Config,
        remote: &Option<String>,
        strategy: &Option<PushStrategy>,
    ) -> Self {
        let default_remote = config.remote_storage();
        let default_remote = remote
            .as_ref()
            .and_then(|f| Some(default_remote.set_storage(&f)))
            .unwrap();
        let default_remote = strategy
            .as_ref()
            .and_then(|f| Some(default_remote.set_strategy(&f)))
            .unwrap();
        self.remote = Some(default_remote);
        self
    }
    pub fn set_comparaison(
        mut self,
        techniques: &Vec<ComparaisonTechnique>,
        path: &Option<PathBuf>,
    ) -> Self {
        self.comparaison = Some(Comparaison::new(techniques, path));
        self
    }

    fn to_facade(&self, path: &PathBuf) -> FileFacade {
        let file = File::new(&path, &self.branch, &self.history, self.timestamp);
        FileFacade::new(file)
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
            Some(self.to_facade(&p))
        })
    }
}

#[derive(Debug, Default)]
pub struct File {
    path: PathBuf,
    history: String,
    branch: String,
    timestamp: i64,
}

impl File {
    pub fn new(path: &PathBuf, branch: &str, history: &str, timestamp: i64) -> Self {
        Self {
            path: path.to_path_buf(),
            branch: branch.to_string(),
            history: history.to_string(),
            timestamp,
        }
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
}

#[derive(Default)]
pub struct FileFacade {
    changed: bool,
    file: File,
    //TODO: maybe instead of multiple connections we could create only one and save everything in
    //batches
    conn: Option<Connection>,
    remote: Option<Remote>,
    comparaison: Option<Comparaison>,
}

impl FileFacade {
    pub fn new(file: File) -> Self {
        Self {
            file,
            changed: true,
            ..Self::default()
        }
    }

    pub fn original_path(&self) -> PathBuf {
        self.file.original_path()
    }

    pub fn set_local_db(&mut self, logbooks_dir: &PathBuf) -> &mut Self {
        //TODO: will path always be relative?
        let mut db_path = self.file.path.to_str().unwrap().to_string();
        db_path.push_str(".db");
        let final_path = logbooks_dir.clone().join(db_path);
        fs::create_dir_all(final_path.parent().expect("no parent for local db"))
            .expect("unable to craete path to local db");
        let db = Database::open(
            &final_path
                .to_str()
                .expect("unable to conver to str")
                .to_string(),
        )
        .expect("unable to open local db");
        self.conn = Some(db.connect().expect("Unable to connect to local db"));
        self
    }

    fn conn(&self) -> &Connection {
        self.conn.as_ref().unwrap()
    }

    pub fn set_remote(mut self, config: &Config) -> Self {
        //TODO: i've probably forgotten to set remote and comparaison structs in the facade
        self.remote = Some(config.remote_storage());
        self
    }

    pub fn set_comparaison(mut self) -> Self {
        self
    }

    pub fn remote(&self) -> Remote {
        self.remote.as_ref().unwrap().clone()
    }

    pub fn push(&self) -> &Self {
        //self.remote is the default remote storage. If we want to specify a remote from the cli
        // for a given file the in we'll be used instead of the default
        // TODO: save all the files config as the remote and so on.
        push_file(self);
        self
    }

    pub fn previous_version(&self) -> PathBuf {
        //TODO: for now just look in the history dir, later on check the dabbatase and find the
        //previous file even if remote
        //TODO: check if we want to keep this list in memory
        let mut files = self
            .file
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
        self.file.history_dir().join(Path::new(&prev_version))
    }

    pub fn compare(&mut self) -> &mut Self {
        self.changed = self.comparaison.as_ref().unwrap().compare(self);
        self
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }

    pub async fn insert_commit(&self, msg: &str) -> &Self {
        self.conn()
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {}_commits (timestamp TIMESTAMP NOT NULL, message TEXT NOT NULL);",
                    &self.file.branch
                ),
                (),
            )
            .await
            .expect("unable to save movement into file logbook");

        self.conn()
            .execute(
                &format!(
                    "INSERT INTO {}_commits (timestamp, message) VALUES (?1, ?2)",
                    &self.file.branch
                ),
                params![&self.file.timestamp, msg],
            )
            .await
            .expect("unable to save movement into project logbook");
        self
    }
    pub async fn insert_snapshot(&self) -> &Self {
        self.conn()
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (timestamp TIMESTAMP NOT NULL);",
                    &self.file.branch
                ),
                (),
            )
            .await
            .expect("unable to save movement into file logbook");

        self.conn()
            .execute(
                &format!("INSERT INTO {} (timestamp) VALUES (?1)", &self.file.branch),
                params![&self.file.timestamp],
            )
            .await
            .expect("unable to save movement into project logbook");
        self
    }

    pub fn duplicate(&self) -> &Self {
        let file = self.file.original_path();
        let local_path = self.file.history_path();
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
                (file.file.path.to_str().expect("unable to convert to str"), file.file.branch.clone(), event.to_string()),
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
                    file.file.path.to_str().expect("unable to convert to str"),
                    file.file.branch.clone(),
                    file.file.timestamp,
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
