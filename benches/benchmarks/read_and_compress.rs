use super::config;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};

use std::{fs::read_dir, os::unix::fs::MetadataExt, path::PathBuf};
use tokio::{fs::File, io::AsyncReadExt};

use zstd::encode_all;

async fn read_and_compress_loop(path: &PathBuf) {
    let mut file = File::open(path).await.unwrap();
    let mut buffer = vec![0; 10 * 1024 * 1024];

    let start = std::time::Instant::now();
    let progress = ProgressBar::new(file.metadata().await.unwrap().size()).with_message(
        format!("Compressing and uploading {}", path.to_str().unwrap()),
    );
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

async fn handle(path: PathBuf) {
    let mut file = File::open(&path).await.unwrap();
    let mut buffer = vec![0; 10 * 1024 * 1024];

    let start = std::time::Instant::now();
    let progress = ProgressBar::new(file.metadata().await.unwrap().size()).with_message(
        format!("Compressing and uploading {}", &path.to_str().unwrap()),
    );
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

async fn read_multiple_files(paths: Vec<PathBuf>) {
    let mut tasks = Vec::with_capacity(paths.len());
    for op in paths {
        tasks.push(tokio::spawn(handle(op)));
    }

    for task in tasks {
        let _ = task.await;
    }
}

async fn read_glommio(paths: &Vec<PathBuf>) {
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    for path in paths {
        let task = read_and_compress_loop(path);
        tasks.push(task);
    }
    let _ = tasks.collect::<Vec<()>>().await;
}
async fn read_multiple_files_loop(paths: &Vec<PathBuf>) {
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    for path in paths {
        let task = read_and_compress_loop(path);
        tasks.push(task);
    }
    let _ = tasks.collect::<Vec<()>>().await;
}

fn bench(c: &mut Criterion) {
    let mut benchmark = c.benchmark_group("ReadAndCompress");
    config::set_default_benchmark_configs(&mut benchmark);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let paths: Vec<PathBuf> = read_dir("test-data")
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect();

    benchmark.bench_with_input(
        BenchmarkId::new("read_glommio", "paths"),
        &paths,
        |b, p| {
            b.to_async(&rt)
                .iter(|| async { read_glommio(black_box(p)) });
        },
    );

    benchmark.bench_with_input(
        BenchmarkId::new("read_multiple_files", "paths"),
        &paths,
        |b, p| {
            b.to_async(&rt)
                .iter(|| async { read_multiple_files(black_box(p.to_owned())) });
        },
    );

    benchmark.bench_with_input(
        BenchmarkId::new("aread_multiple_files_loop", "paths"),
        &paths,
        |b, p| {
            b.to_async(&rt)
                .iter(|| async { read_multiple_files_loop(black_box(p)) });
        },
    );

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
