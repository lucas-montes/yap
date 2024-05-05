use crate::config::{Author, Config, PushStrategy, RemoteConfig, Storage};
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use libsql::{params, Builder, Connection, Database};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    env,
    fmt::Debug,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{
    cli::Events,
    comparaison::{Comparaison, ComparaisonTechnique},
    remote::{pull_file, push_file},
    versioning::{get_latest_git_commit, Commit},
};

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

    pub async fn local(path: &str) -> Self {
        let db = Builder::new_local(path)
            .build()
            .await
            .expect("unable to open local");
        Self::new(db)
    }

    pub async fn file_is_tracked(&self, file: &FileFacade) -> bool {
        let mut result = self
            .conn()
            .query(
                "SELECT EXISTS(SELECT 1 FROM files WHERE path=?1 AND branch=?2 LIMIT 1);",
                params![
                    file.file.path.to_str().expect("unable to convert to str"),
                    file.file.branch.clone(),
                ],
            )
            .await
            .expect("error checking if the files is already tracked");

        0 != result
            .next()
            .await
            .expect("iterator empty")
            .expect("empyt")
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
            .unwrap_or_else(|_| {
                panic!(
                    "error saving the event: {:?} for the file {:?}",
                    event, file
                )
            });
    }

    pub async fn track_file(&self, file: &FileFacade) {
        self.conn()
            .execute(
                "INSERT INTO files (path, branch) VALUES (?1, ?2)",
                params![
                    file.path().to_str().expect("unable to convert to str"),
                    file.branch()
                ],
            )
            .await
            .expect("error erting the newly tracked file");
    }

    pub async fn files_tracked(&self) -> Vec<String> {
        self.db
            .connect()
            .unwrap()
            .query("SELECT path FROM files", ())
            .await
            .unwrap()
            .into_stream()
            .map(|f| f.unwrap().get::<String>(0).unwrap())
            .collect()
            .await
    }
}

//  TODO: pass more things as ref
#[derive(Debug, Default)]
pub struct FileFacadeFactory {
    branch: String,
    history_dir: String,
    logbooks_dir: PathBuf,
    timestamp: i64,
    author: Author,
    remote: Option<RemoteConfig>,
    comparaison: Option<Comparaison>,
    message: Option<String>,
    stack: VecDeque<PathBuf>,
    multi_progress_bar: MultiProgress,
}

impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str, config: &Config) -> Self {
        Self {
            branch: branch.to_owned(),
            history_dir: config.history_dir().to_owned(),
            logbooks_dir: config.logbooks_dir().to_owned(),
            author: config.author(),
            timestamp: chrono::offset::Local::now().timestamp(),
            stack: VecDeque::from(paths),
            multi_progress_bar: MultiProgress::new(),
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
            default_remote.storage = rem.to_owned();
        }
        if let Some(strategy) = strategy {
            default_remote.strategy = strategy.to_owned();
        }
        self.remote = Some(default_remote);
        self
    }

    pub fn set_comparaison(
        mut self,
        technique: &ComparaisonTechnique,
        path: &Option<PathBuf>,
    ) -> Self {
        self.comparaison = Some(Comparaison::new(technique, path));
        self
    }

    pub fn set_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_owned());
        self
    }

    fn to_facade(&self, path: &Path) -> FileFacade {
        let file = File::new(
            path,
            &self.branch,
            &self.history_dir,
            self.timestamp,
            self.author.clone(),
        );
        let logbook = FileLogbook::new(path, &self.logbooks_dir);
        let progress_bar = self.multi_progress_bar.add(ProgressBar::new(0));
        let facade = FileFacade::new(file)
            .set_logbook(logbook)
            .set_progress_bar(progress_bar);
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

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Remote {
    pk: Option<u32>,
    path: PathBuf,
    strategy: PushStrategy,
    storage: Storage,
}

impl Remote {
    pub fn new(path: PathBuf, strategy: PushStrategy, storage: Storage) -> Self {
        Self {
            pk: None,
            path,
            strategy,
            storage,
        }
    }

    pub fn get(_path: PathBuf, _storage: Storage) -> Self {
        todo!()
    }
}

impl LogbookProvider for Remote {
    async fn query(&self) -> String {
        "INSERT INTO remotes (path, storage, strategy) VALUES (?1, ?2, ?3)".to_string()
    }
    async fn params(&self) -> Vec<String> {
        vec![
            self.path.to_str().unwrap().to_owned(),
            self.storage.to_string(),
            self.strategy.to_string(),
        ]
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct File {
    pk: Option<u32>,
    author: Author,
    remote: Option<Remote>,
    path: PathBuf,
    history: String,
    branch: String,
    timestamp: i64,
}

impl File {
    pub fn new(
        path: &Path,
        branch: &str,
        history: &str,
        timestamp: i64,
        author: Author,
    ) -> Self {
        Self {
            pk: None,
            path: path.to_path_buf(),
            branch: branch.to_string(),
            history: history.to_string(),
            timestamp,
            author,
            remote: None,
        }
    }

    fn set_remote(&mut self, remote: Remote) -> &Self {
        self.remote = Some(remote);
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
        //TODO: will path always be relative? probably not
        self.history_dir()
            .join(Path::new(&self.timestamp.to_string()))
    }
}

impl LogbookProvider for File {
    async fn query(&self) -> String {
        "INSERT INTO files (timestamp, path, branch, author) VALUES (?1, ?2, ?3, ?4)"
            .to_string()
    }
    async fn params(&self) -> Vec<String> {
        vec![
            self.timestamp.to_string(),
            self.path.to_str().unwrap().to_owned(),
            self.branch.to_owned(),
            self.author.pk(), //TODO: add the authors all over the place
        ]
    }
}

#[derive(Default, Debug)]
pub struct FileLogbook {
    db: Option<Database>,
    path: PathBuf,
}

impl FileLogbook {
    pub fn new(file_path: &Path, logbooks_dir: &Path) -> Self {
        //TODO: will path always be relative?
        let mut db_path = file_path.to_str().unwrap().to_string();
        db_path.push_str(".db");
        Self {
            db: None,
            path: logbooks_dir.join(db_path),
        }
    }

    pub async fn init(&mut self) -> &Self {
        tokio::fs::create_dir_all(self.path.parent().expect("no parent for local db"))
            .await
            .expect("unable to craete path to local db");
        let db = Builder::new_local(self.path.to_str().expect("unable to conver to str"))
            .build()
            .await
            .expect("unable to open local db");
        self.db = Some(db);
        self
    }

    pub async fn create(&self) -> &Self {
        let query = match tokio::fs::read_to_string("schemas/file_logbook.sql").await {
            Ok(sql) => sql,
            Err(err) => {
                panic!("{err}");
            },
        };
        self.conn()
            .await
            .execute_batch(&query)
            .await
            .expect("unable to create tables into file logbook");
        self
    }

    pub async fn insert<'a, T: LogbookProvider + Debug>(&self, object: &'a T) -> &'a T {
        //TODO: maybe return the branch as a struct
        self.conn()
            .await
            .execute(&object.query().await, object.params().await)
            .await
            .unwrap_or_else(|_| {
                panic!("unable to save movement into project logbook {:?}", object)
            });
        object
    }

    async fn conn(&self) -> Connection {
        self.db
            .as_ref()
            .unwrap()
            .connect()
            .expect("Unable to connect to local db")
    }
}

pub trait LogbookProvider {
    async fn query(&self) -> String;
    async fn params(&self) -> Vec<String>;
}

fn progress_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.blue} {msg}")
        .unwrap()
        .tick_strings(&[
            "▹▹▹▹▹",
            "▸▹▹▹▹",
            "▹▸▹▹▹",
            "▹▹▸▹▹",
            "▹▹▹▸▹",
            "▹▹▹▹▸",
            "▪▪▪▪▪",
        ])
}
fn progress_bar_style() -> ProgressStyle {
    ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes:>7}/{total_bytes:7} [{bytes_per_sec}] {msg}\n")
            .unwrap()
            .progress_chars("->#")
}

#[derive(Debug)]
pub struct FileFacade {
    changed: bool,
    file: File,
    logbook: FileLogbook,
    remote: Option<RemoteConfig>,
    comparaison: Option<Comparaison>,
    progress_bar: Option<ProgressBar>,
}

impl FileFacade {
    pub fn new(file: File) -> Self {
        Self {
            file,
            changed: true,
            logbook: FileLogbook::default(),
            remote: None,
            comparaison: None,
            progress_bar: None,
        }
    }

    pub async fn init(mut self) -> Self {
        self.logbook.init().await;
        self
    }

    pub fn set_progress_bar(mut self, progress_bar: ProgressBar) -> Self {
        progress_bar.set_style(progress_bar_style());
        self.progress_bar = Some(progress_bar);
        self
    }

    pub fn file(&self) -> File {
        self.file.clone()
    }

    pub fn branch(&self) -> &str {
        &self.file.branch
    }

    pub fn path(&self) -> &Path {
        &self.file.path
    }

    pub fn original_path(&self) -> PathBuf {
        self.file.original_path()
    }

    pub fn remote(&self) -> RemoteConfig {
        self.remote.as_ref().unwrap().clone()
    }

    pub fn set_remote(mut self, remote: &RemoteConfig) -> Self {
        self.remote = Some(remote.clone());
        self
    }

    pub fn set_comparaison(mut self, comparaison: &Comparaison) -> Self {
        self.comparaison = Some(comparaison.clone());
        self
    }

    pub fn set_logbook(mut self, logbook: FileLogbook) -> Self {
        self.logbook = logbook;
        self
    }

    fn comparaison(&self) -> Comparaison {
        self.comparaison.as_ref().unwrap().clone()
    }

    pub async fn add(self) -> Self {
        let original = self.file.original_path();
        let duplicata = self.file.history_path();

        match self.progress_bar.clone() {
            Some(progress_bar) => {
                progress_bar.set_message(format!(
                    "Copying {:?} into {:?}",
                    &original, &duplicata
                ));
                progress_bar.set_length(original.metadata().unwrap().size());
                self.duplicate(&original, &duplicata).await;
                progress_bar.enable_steady_tick(Duration::from_millis(120));
                progress_bar.set_style(progress_spinner_style());
                progress_bar.set_message("Creating the logbook");
                self.logbook.create().await;
                progress_bar.set_message("Saving the movement");
                self.logbook.insert(&self.file).await;
                progress_bar.set_style(ProgressStyle::with_template("{msg}").unwrap());
                progress_bar.finish_with_message(format!("{:?} saved ✅", &original));
            },
            None => {
                self.duplicate(&original, &duplicata).await;
                self.logbook.create().await;
                self.logbook.insert(&self.file).await;
            },
        };
        self
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }

    pub async fn compare(&self, msg: &str) -> &Self {
        //TODO: if commiting we see that the file hasn't changed we may want to
        //use a symlink
        let previous = self.previous_version();
        let original = self.file.original_path();
        let duplicata = self.file.history_path();
        let previous_file = previous.file();
        let mut comp = self.comparaison();
        let current_file = self.file();

        match self.progress_bar.clone() {
            Some(progress_bar) => {
                progress_bar.enable_steady_tick(Duration::from_millis(120));
                progress_bar.set_style(progress_spinner_style());
                progress_bar
                    .set_message(format!("Comparing versions for {:?}", &original));

                let diff = tokio::task::spawn_blocking(move || {
                    comp.compare(&current_file, &previous_file)
                })
                .await
                .unwrap();

                progress_bar.set_style(progress_bar_style());
                progress_bar.set_message(format!(
                    "Copying {:?} into {:?}",
                    &original, &duplicata
                ));
                progress_bar.set_length(original.metadata().unwrap().size());
                self.duplicate(&original, &duplicata).await;

                progress_bar.enable_steady_tick(Duration::from_millis(120));
                progress_bar.set_style(progress_spinner_style());
                progress_bar.set_message("Saving the result with the differences");
                self.logbook.insert(&diff).await;
                let git_commit = get_latest_git_commit().await;
                progress_bar.set_message("Saving commit into the logbook");
                let commit = Commit::new(
                    self.file.branch.to_owned(),
                    self.file.original_path(),
                    previous.file.original_path(),
                    msg.to_owned(),
                    self.file.author.to_owned(),
                )
                .set_diff(&diff)
                .set_git_commit(git_commit);
                self.logbook.insert(&commit).await;
                progress_bar.set_style(ProgressStyle::with_template("{msg}").unwrap());
                progress_bar.finish_with_message(format!("{:?} commited ✅", &original));
            },
            None => {
                let diff = self.comparaison().compare(&self.file(), &previous.file());
                self.duplicate(&original, &duplicata).await;

                self.logbook.insert(&diff).await;
                let git_commit = get_latest_git_commit().await;
                let commit = Commit::new(
                    self.file.branch.clone(),
                    self.file.original_path(),
                    previous.file.original_path(),
                    msg.to_owned(),
                    self.file.author.clone(),
                )
                .set_diff(&diff)
                .set_git_commit(git_commit);
                self.logbook.insert(&commit).await;
            },
        };
        self
    }

    pub async fn remove(self) -> Self {
        let remote = self.remote();
        let operator = remote.get_storage_operator();
        operator
            .delete(self.path().to_str().unwrap())
            .await
            .unwrap();
        self
    }

    pub async fn push(mut self) -> Self {
        let remote = push_file(&self).await;
        self.logbook.insert(&remote).await;
        self.file.set_remote(remote);
        self.logbook.insert(&self.file).await;
        //TODO: push the comparaison results
        //TODO: it would be cool to save the errors if any from the copies and so on and save them
        // in the db
        self
    }

    pub async fn pull(mut self) -> Self {
        let remote = pull_file(&self).await;
        self.logbook.insert(&remote).await;
        self.file.set_remote(remote);
        self.logbook.insert(&self.file).await;
        self
    }

    // Utils
    pub fn previous_version(&self) -> FileFacade {
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
        let prev_timestamp = files.pop().unwrap();
        let file = File::new(
            &self.file.path,
            &self.file.branch,
            &self.file.history,
            prev_timestamp,
            self.file.author.clone(),
        );
        FileFacade::new(file)
    }

    pub async fn duplicate(&self, original: &Path, duplicata: &Path) -> &Self {
        tokio::fs::create_dir_all(
            duplicata
                .parent()
                .expect("unable to get duplciate local path parent"),
        )
        .await
        .expect("Couldn't create dirs");
        let progress_bar = self.progress_bar.as_ref().unwrap();

        let mut origin_file = tokio::fs::File::open(&original).await.unwrap();
        let mut duplicated_file = tokio::fs::File::create(&duplicata).await.unwrap();
        let mut buffer = vec![0; 32 * 1_024];

        loop {
            let bytes_read = match origin_file.read(&mut buffer).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading from file: {}", e);
                    break;
                },
            };
            if bytes_read == 0 {
                break;
            }
            let bytes_wrote = duplicated_file.write(&buffer[..bytes_read]).await.unwrap();
            progress_bar.inc(bytes_wrote as u64);
        }
        self
    }
}
