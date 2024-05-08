use super::config;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::{fs::read_dir, io::Read, os::unix::fs::MetadataExt, path::PathBuf};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use zstd::encode_all;

const O_DIRECT: i32 = 0x4000;

async fn read_and_compress(path: &PathBuf, progress: ProgressBar) {
    let mut file = File::open(&path).await.unwrap();
    let mut buffer = vec![0; 10 * 1024 * 1024];
    let mut file2 = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(&format!("dump/a/{}", path.to_str().unwrap()))
        .await
        .expect("Can't open");
    let start = std::time::Instant::now();

    loop {
        let bytes_read = match file.read(&mut buffer).await.and_then(|n| {
            n.eq(&0)
                .then(|| n)
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, ""))
        }) {
            Ok(n) => n,
            Err(e) => {
                break;
            },
        };
        file2
            .write_all(&encode_all(&buffer[..bytes_read], 0).unwrap())
            .await;
        progress.inc(bytes_read as u64);
    }
    progress
        .with_elapsed(start.elapsed())
        .finish_with_message("File uploaded");
}

async fn read_and_compress_modif(path: &PathBuf) {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECT)
        .open(&path)
        .await
        .expect("Can't open");
    let mut file2 = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .custom_flags(O_DIRECT)
        .open(&format!("dump/b/{}", path.to_str().unwrap()))
        .await
        .expect("Can't open");
    let mut buffer = vec![0; 10 * 1024 * 1024];

    loop {
        let bytes_read = match file.read(&mut buffer).await {
            Ok(n) => n,
            Err(e) => {
                break;
            },
        };
        if bytes_read == 0 {
            break;
        }
        file2
            .write_all(&encode_all(&buffer[..bytes_read], 0).unwrap())
            .await;
    }
}

async fn read_multiple_files_modif(paths: &Vec<PathBuf>) {
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();

    for path in paths {
        let task = read_and_compress_modif(path);
        tasks.push(task);
    }
    let _ = tasks.collect::<Vec<()>>().await;
}

async fn read_multiple_files(paths: &Vec<PathBuf>) {
    let tasks: FuturesUnordered<_> = FuturesUnordered::new();
    let m = MultiProgress::new();

    for path in paths {
        let pb = ProgressBar::new(path.metadata().unwrap().size()).with_message(format!(
            "Compressing and uploading {}",
            &path.to_str().unwrap()
        ));
        let progress = m.add(pb);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Starting to read file");
        let task = read_and_compress(path, progress);
        tasks.push(task);
    }
    let _ = tasks.collect::<Vec<()>>().await;
}

fn bench(c: &mut Criterion) {
    let mut benchmark = c.benchmark_group("ReadAndCompress");
    config::set_default_benchmark_configs(&mut benchmark);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let paths: Vec<PathBuf> = read_dir("test_data")
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect();

    benchmark.bench_with_input(
        BenchmarkId::new("read_multiple_files", "paths"),
        &paths,
        |b, p| {
            b.to_async(&rt)
                .iter(|| async { read_multiple_files(black_box(p)) });
        },
    );

    benchmark.bench_with_input(
        BenchmarkId::new("read_multiple_files_modif", "paths"),
        &paths,
        |b, p| {
            b.to_async(&rt)
                .iter(|| async { read_multiple_files_modif(black_box(p)) });
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
