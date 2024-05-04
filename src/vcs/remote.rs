use crate::vcs::{FileFacade, Remote};

use indicatif::{ProgressBar, ProgressStyle};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn push_file(file: &FileFacade) -> Remote {
    let remote = file.remote();
    let start = std::time::Instant::now();
    let _operator = remote.get_storage_operator();
    let file_size = file.original_path().metadata().unwrap().len();
    let progress = ProgressBar::new(file_size).with_message("Uploading file");
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes:>7}/{total_bytes:7} [{bytes_per_sec}] {msg}\n")
            .unwrap()
            .progress_chars("#->"),
    );
    let data = tokio::fs::read(file.original_path()).await.unwrap();
    let _result = zstd::bulk::compress(&data, 0).unwrap();

    let mut file_content = File::open(file.original_path()).await.unwrap();
    let mut buffer = vec![0; 5 * 1024 * 1024];
    loop {
        let bytes_read = match file_content.read(&mut buffer).await {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Error reading from file: {}", e);
                break;
            },
        };
        if bytes_read == 0 {
            break;
        }
        progress.inc(bytes_read.try_into().unwrap());
        zstd::stream::encode_all(&buffer[..bytes_read], 0).unwrap();
        // operator
        //     .write(file.path().to_str().unwrap(), result)
        //     .await
        //     .unwrap();
    }

    progress
        .with_elapsed(start.elapsed())
        .finish_with_message("File uploaded");

    Remote::new(file.path().to_path_buf(), remote.strategy, remote.storage)
}

pub async fn pull_file(file: &FileFacade) -> Remote {
    // TODO: when pushing and pulling from remote, how to know which files to get and send? keep
    // the strategy think? probably no. How should we save the files on the remote? as they are in
    // the history_path? probably.
    if file.path().exists() {
        println!("Already exists: {:?}", file.path())
    }
    let remote = file.remote();
    let result = file
        .remote()
        .get_storage_operator()
        .read(file.path().to_str().unwrap())
        .await
        .unwrap();
    tokio::fs::create_dir_all(&file.path().parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&file.path(), &result).await.unwrap();
    println!("{:?} downloaded", file.path());
    Remote::get(file.path().to_path_buf(), remote.storage)
}
