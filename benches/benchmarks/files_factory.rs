use futures::stream::FuturesUnordered;
use futures_util::StreamExt;
use std::{
    collections::VecDeque,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    str::FromStr,
};

use super::config;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{fs::File, io::AsyncReadExt};
use zstd::encode_all;
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct FileFacade {
    path: PathBuf,
    branch: String,
    history_dir: String,
    logbooks_dir: String,
    timestamp: i64,
    changed: bool,
}
#[allow(dead_code)]
impl FileFacade {
    pub async fn new(
        path: PathBuf,
        branch: String,
        history_dir: String,
        logbooks_dir: String,
        timestamp: i64,
    ) -> Self {
        Self {
            path,
            branch,
            history_dir,
            logbooks_dir,
            timestamp,
            changed: false,
        }
    }

    pub async fn commit(&self, msg: &str) -> &Self {
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        let _ = msg.to_owned();
        self
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }
}

#[derive(Default, Debug)]
pub struct FileFacadeFactory {
    branch: String,
    history_dir: String,
    logbooks_dir: String,
    timestamp: i64,
    remote: Option<String>,
    comparaison: Option<String>,
    stack: VecDeque<PathBuf>,
}
#[allow(dead_code)]
impl FileFacadeFactory {
    pub fn new(paths: Vec<PathBuf>, branch: &str) -> Self {
        Self {
            branch: branch.to_owned(),
            history_dir: branch.to_owned(),
            logbooks_dir: branch.to_owned(),
            timestamp: chrono::offset::Local::now().timestamp(),
            stack: VecDeque::from(paths),
            ..Self::default()
        }
    }

    async fn ato_facade(&self, path: &Path) -> FileFacade {
        let facade = FileFacade::new(
            path.to_owned(),
            self.branch.to_owned(),
            self.logbooks_dir.to_owned(),
            self.history_dir.to_owned(),
            self.timestamp,
        )
        .await;
        match (self.comparaison.is_some(), self.remote.is_some()) {
            // We are just adding files
            (false, false) => facade,
            // IDK what this could be
            _ => todo!("why have we comparaison and remote"),
        }
    }

    fn to_facade(&self, path: &Path) -> FileFacade {
        let facade = FileFacade {
            path: path.to_owned(),
            branch: self.branch.to_owned(),
            logbooks_dir: self.logbooks_dir.to_owned(),
            history_dir: self.history_dir.to_owned(),
            timestamp: self.timestamp,
            changed: false,
        };
        match (self.comparaison.is_some(), self.remote.is_some()) {
            // We are just adding files
            (false, false) => facade,
            // IDK what this could be
            _ => todo!("why have we comparaison and remote"),
        }
    }

    pub async fn anext(&mut self) -> Option<FileFacade> {
        if let Some(p) = self.stack.pop_front() {
            if p.is_dir() {
                while let Some(f) = tokio::fs::read_dir(&p)
                    .await
                    .unwrap()
                    .next_entry()
                    .await
                    .unwrap()
                {
                    self.stack.push_back(f.path());
                }
            }
            Some(self.ato_facade(&p).await)
        } else {
            None
        }
    }

    pub async fn new_approach(&mut self) {
        let tasks: FuturesUnordered<_> = FuturesUnordered::new();
        self.stack.pop_front().map(|p| {
            if p.is_dir() {
                p.read_dir()
                    .unwrap()
                    .for_each(|p| self.stack.push_back(p.unwrap().path()));
            }
            let path = self.to_facade(&p);
            tasks.push(handle_file(path));
        });
        let _: Vec<()> = tasks.collect().await;
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

async fn handle_file(file_facade: FileFacade) {
    if file_facade.commit("self.message").await.has_changed() {
        let mut file = File::open(&file_facade.path).await.unwrap();
        let mut buffer = vec![0; 10 * 1024 * 1024];

        let start = std::time::Instant::now();
        let progress = ProgressBar::new(file.metadata().await.unwrap().size())
            .with_message(format!(
                "Compressing and uploading {}",
                file_facade.path.to_str().unwrap()
            ));
        progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Starting to read file");
        loop {
            let bytes_read = match file.read(&mut buffer).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading from file: {}", e);
                    break;
                },
            };
            if bytes_read == 0 {
                break;
            }
            encode_all(&buffer[..bytes_read], 0).unwrap();
        }
        progress.finish_with_message("Starting to write file to remote");

        progress
            .with_elapsed(start.elapsed())
            .finish_with_message("File uploaded");
    }
}
#[allow(dead_code)]
async fn arun(paths: Vec<PathBuf>) -> i16 {
    let mut files = FileFacadeFactory::new(paths, "master");

    while let Some(f) = files.anext().await {
        handle_file(f).await;
    }
    0
}
#[allow(dead_code)]
async fn new_approach(paths: Vec<PathBuf>) -> i16 {
    FileFacadeFactory::new(paths, "master").new_approach().await;
    0
}
#[allow(dead_code)]
async fn run(paths: Vec<PathBuf>) -> i16 {
    let mut files = FileFacadeFactory::new(paths, "master");

    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    while let Some(path) = files.next() {
        tasks.push(handle_file(path));
    }
    let _: Vec<()> = tasks.collect().await;
    0
}
#[allow(dead_code)]
fn r(paths: &mut Vec<PathBuf>, files: &FileFacadeFactory) -> Option<FileFacade> {
    paths.pop().map(|p| {
        if p.is_dir() {
            p.read_dir()
                .unwrap()
                .for_each(|p| paths.push(p.unwrap().path()));
        }
        files.to_facade(&p)
    })
}
#[allow(dead_code)]
async fn parse_recursive(mut paths: Vec<PathBuf>, files: &FileFacadeFactory) {
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    while let Some(f) = r(&mut paths, files) {
        tasks.push(handle_file(f));
    }
    let _: Vec<()> = tasks.collect().await;
}
#[allow(dead_code)]
async fn parse_stack(paths: Vec<PathBuf>, files: &FileFacadeFactory) {
    let mut stack = VecDeque::from(paths);
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    stack.pop_front().map(|p| {
        if p.is_dir() {
            p.read_dir()
                .unwrap()
                .for_each(|p| stack.push_back(p.unwrap().path()));
        }
        let path = files.to_facade(&p);
        tasks.push(handle_file(path));
    });
    let _: Vec<()> = tasks.collect().await;
}
#[allow(dead_code)]
fn bench(c: &mut Criterion) {
    let mut benchmark = c.benchmark_group("Custom iterator");
    config::set_default_benchmark_configs(&mut benchmark);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let paths: Vec<PathBuf> = vec![
        PathBuf::from_str("test-data-dir").unwrap(),
        PathBuf::from_str("test-data").unwrap(),
    ];
    let paths2 = paths.clone();
    let files = FileFacadeFactory::new(paths.clone(), "master");
    let files2 = FileFacadeFactory::new(paths2.clone(), "master");

    benchmark.bench_with_input(
        BenchmarkId::new("parse_recursive", "paths"),
        &(paths, files),
        |b, (p, f)| {
            b.to_async(&rt)
                .iter(|| async { parse_recursive(black_box(p.to_vec()), black_box(&f)) });
        },
    );

    benchmark.bench_with_input(
        BenchmarkId::new("stack", "paths"),
        &(paths2, files2),
        |b, (p, f)| {
            b.to_async(&rt)
                .iter(|| async { parse_stack(black_box(p.to_vec()), black_box(&f)) });
        },
    );

    // benchmark.bench_with_input(
    //     BenchmarkId::new("new_approach", "paths"),
    //     &paths,
    //     |b, p| {
    //         b.to_async(&rt)
    //             .iter(|| async { new_approach(black_box(p.to_vec())) });
    //     },
    // );

    // benchmark.bench_with_input(BenchmarkId::new("arun", "paths"), &paths, |b, p| {
    //     b.to_async(&rt)
    //         .iter(|| async { arun(black_box(p.to_vec())) });
    // });

    // benchmark.bench_with_input(BenchmarkId::new("run", "paths"), &paths, |b, p| {
    //     b.to_async(&rt)
    //         .iter(|| async { run(black_box(p.to_vec())) });
    // });

    benchmark.finish()
}

#[cfg(not(target_os = "windows"))]
criterion_group! {
    name = benches;
    config = config::get_default_profiling_configs();
    targets = bench
}
#[cfg(target_os = "windows")]
criterion_group!(benches, bench);

criterion_main!(benches);
