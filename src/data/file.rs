use libsql::{params, Connection, Database};
use std::{
    collections::VecDeque,
    env, fs,
    path::{Path, PathBuf},
};
use turso::RowsIter;

use crate::{
    config::Config,
    remote::{push_file, Remote, Storage},
};
use crate::{enums::Events, remote::PushStrategy};

use super::comparaison::{Comparaison, ComparaisonTechnique};

pub struct Logbook {
    db: Database,
}

impl Logbook {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn conn(&self) -> Connection {
        self.db.connect().expect("cant connect to db")
    }

    pub fn local(path: &str) -> Self {
        let db = Database::open(path).expect("unable to open local");
        Self::new(db)
    }

    pub async fn file_is_tracked(&self, file: &FileFacade) -> bool {
        let mut result = self
            .conn()
            .query(
                "SELECT EXISTS(SELECT 1 FROM files WHERE path=?1 AND branch=?2 LIMIT 1);",
                (
                    file.file.path.to_str().expect("unable to convert to str"),
                    file.file.branch.clone(),
                ),
            )
            .await
            .expect("error executing insert");
        0 != RowsIter::new(&mut result)
            .next()
            .expect("iterator empty")
            .get::<u32>(0)
            .expect("couldnt get the value")
    }

    pub async fn save_event(&self, file: &FileFacade, event: &Events) {
        self.conn()
            .execute(
                "INSERT INTO events (timestamp, branch, path, event) VALUES (?1, ?2, ?3, ?4)",
                params![
                    file.file.timestamp,
                    file.file.branch.clone(),
                    file.file.path.to_str().expect("unable to convert to str"),
                    event.to_string()
                ],
            )
            .await
            .expect("error executing insert");
    }

    pub async fn track_file(&self, path: &Path) {
        self.conn()
            .execute(
                "INSERT INTO files (path) VALUES (?1)",
                params![path.to_str().expect("unable to convert to str"),],
            )
            .await
            .expect("error executing insert");
    }
}

//  TODO: pass more things as ref
#[derive(Debug, Default)]
pub struct FileFacadeFactory {
    branch: String,
    history_dir: String,
    logbooks_dir: PathBuf,
    timestamp: i64,
    remote: Option<Remote>,
    comparaison: Option<Comparaison>,
    stack: VecDeque<PathBuf>,
}

impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str, config: &Config) -> Self {
        Self {
            branch: branch.to_owned(),
            history_dir: config.history_dir().to_owned(),
            logbooks_dir: config.logbooks_dir().to_owned(),
            timestamp: chrono::offset::Local::now().timestamp(),
            stack: VecDeque::from(paths),
            ..Self::default()
        }
    }

    pub fn set_remote(
        mut self,
        config: &Config,
        remote: &Option<Storage>,
        strategy: &Option<PushStrategy>,
    ) -> Self {
        let mut default_remote = config.remote_storage();
        if let Some(rem) = remote {
            default_remote = default_remote.set_storage(rem);
        }
        if let Some(strategy) = strategy {
            default_remote = default_remote.set_strategy(strategy);
        }
        self.remote = Some(default_remote);
        self
    }

    pub fn set_comparaison(
        mut self,
        techniques: &[ComparaisonTechnique],
        path: &Option<PathBuf>,
    ) -> Self {
        self.comparaison = Some(Comparaison::new(techniques, path));
        self
    }

    fn to_facade(&self, path: &Path) -> FileFacade {
        let file = File::new(path, &self.branch, &self.history_dir, self.timestamp);
        let logbook = FileLogbook::new(path, &self.logbooks_dir);
        let facade = FileFacade::new(file).set_logbook(logbook);
        match (self.comparaison.is_some(), self.remote.is_some()) {
            // We are pushing so we need the remote info
            (false, true) => facade.set_remote(self.remote.as_ref().unwrap()),
            // We are commiting so we need the comparaison info
            (true, false) => facade.set_comparaison(self.comparaison.as_ref().unwrap()),
            // We are just adding files
            (false, false) => facade,
            // IDK what this could be
            _ => todo!("why have we comparaison and remote"),
        }
    }
}

impl Iterator for FileFacadeFactory {
    type Item = FileFacade;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop_front().map(|p| {
            if p.is_dir() {
                p.read_dir()
                    .unwrap()
                    .for_each(|p| self.stack.push_back(p.unwrap().path()));
            }
            self.to_facade(&p)
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
    pub fn new(path: &Path, branch: &str, history: &str, timestamp: i64) -> Self {
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
struct FileLogbook {
    //TODO: maybe instead of multiple connections we could create only one and save everything in
    //batches
    conn: Option<Connection>,
}

impl FileLogbook {
    pub fn new(file_path: &Path, logbooks_dir: &Path) -> Self {
        //TODO: will path always be relative?
        let mut db_path = file_path.to_str().unwrap().to_string();
        db_path.push_str(".db");
        let final_path = logbooks_dir.join(db_path);
        fs::create_dir_all(final_path.parent().expect("no parent for local db"))
            .expect("unable to craete path to local db");
        let db = Database::open(
            final_path
                .to_str()
                .expect("unable to conver to str")
                .to_string(),
        )
        .expect("unable to open local db");
        Self {
            conn: Some(db.connect().expect("Unable to connect to local db")),
        }
    }

    pub async fn create_tables(&self) -> &Self {
        let query = match tokio::fs::read_to_string("migrations/file_logbook.sql").await {
            Ok(sql) => sql,
            Err(err) => {
                panic!("{err}");
            }
        };
        self.conn()
            .execute(&query, ())
            .await
            .expect("unable to create tables into file logbook");
        self
    }

    pub async fn create_branch(&self, branch: &str, description: &str) {
        self.conn()
            .execute(
                "INSERT INTO branches (name, description) VALUES (?1, ?2)",
                params![timestamp, msg],
            )
            .await
            .expect("unable to save movement into project logbook");
        self
    }

    pub async fn insert_commit(&self, branch: &str, timestamp: &i64, msg: &str) -> &Self {
        self.conn()
            .execute(
                &format!(
                    "INSERT INTO {}_commits (timestamp, message) VALUES (?1, ?2)",
                    branch
                ),
                params![timestamp, msg],
            )
            .await
            .expect("unable to save movement into project logbook");
        self
    }

    pub async fn insert_snapshot(&self, branch: &str, timestamp: &i64) -> &Self {
        // Create a table for the branch if it doesnt exists
        // Save the new branch in the
        self.conn()
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (timestamp TIMESTAMP NOT NULL);",
                    &branch
                ),
                (),
            )
            .await
            .expect("unable to save movement into file logbook");

        self.conn()
            .execute(
                &format!("INSERT INTO {} (timestamp) VALUES (?1)", &branch),
                params![timestamp],
            )
            .await
            .expect("unable to save movement into project logbook");
        self
    }

    fn conn(&self) -> &Connection {
        self.conn.as_ref().unwrap()
    }
}

#[derive(Default)]
pub struct FileFacade {
    changed: bool,
    file: File,
    logbook: FileLogbook,
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

    pub fn set_logbook(mut self, logbook: FileLogbook) -> Self {
        self.logbook = logbook;
        self
    }

    pub async fn keep_track(&self) -> &Self {
        self.duplicate();
        self.logbook.create_tables();
        self
    }

    pub async fn commit(&self, msg: &str) -> &Self {
        self
    }

    pub fn history_path(&self) -> PathBuf {
        self.file.history_path()
    }

    pub fn path(&self) -> &Path {
        &self.file.path
    }

    pub fn original_path(&self) -> PathBuf {
        self.file.original_path()
    }

    pub fn set_remote(mut self, remote: &Remote) -> Self {
        self.remote = Some(remote.clone());
        self
    }

    pub fn set_comparaison(mut self, comparaison: &Comparaison) -> Self {
        self.comparaison = Some(comparaison.clone());
        self
    }

    pub fn remote(&self) -> Remote {
        self.remote.as_ref().unwrap().clone()
    }

    pub async fn push(&self) -> &Self {
        //self.remote is the default remote storage. If we want to specify a remote from the cli
        // for a given file the in we'll be used instead of the default
        // TODO: save all the files config as the remote and so on.
        push_file(self).await;
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
        fs::create_dir_all(local_path.parent().unwrap()).unwrap();
        match fs::copy(file, &local_path) {
            Ok(_) => self,
            Err(err) => panic!("{}", err),
        }
    }
}
