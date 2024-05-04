use std::{collections::HashMap, path::PathBuf};

use clap::Args;
use serde::Deserialize;
use serde_yaml::Mapping;
use shlex::split;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct Pipeline {
    // Handle foerach on top of stages
    stages: HashMap<String, Stage>,
}

impl Pipeline {
    #[allow(dead_code)]
    fn build_graph(self) {
        let mut stage_graph: HashMap<u8, Vec<Stage>> = HashMap::default();
        for (_k, stage) in self.stages {
            if stage.deps.is_empty() {
                //Handle more complex logic
                stage_graph.insert(0, vec![stage]);
            } else {
                todo!()
            }
        }
    }
}
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Stage {
    cmd: String,
    wdir: String,
    #[serde(default)]
    deps: Vec<String>,
    outs: Vec<String>,
    metrics: Vec<HashMap<String, Metric>>,
}

const CACHE_DEFAULT: bool = true;
fn cache_default() -> bool {
    CACHE_DEFAULT
}
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Metric {
    #[serde(rename = "cache", default = "cache_default")]
    cache: bool,
}

#[derive(Debug, Args)]
pub struct ReproArgs {
    #[arg(short, long)]
    paths: Vec<PathBuf>,
}

pub async fn repro(args: &ReproArgs) -> i16 {
    let p = &args.paths[0];
    run_pipeline(p.to_path_buf()).await;
    0
    //let mut cmd = build_command();
    //execute_command(&mut cmd).await
}

async fn run_pipeline(path: PathBuf) {
    let mut full_path = parse_path(path);
    let pipeline = parse_stages(&full_path);
    let cmd = &pipeline.stages["first_stage"].cmd;

    full_path.set_file_name("params.yaml");
    let params: Mapping = read_yaml(&full_path);
    let mut final_cmd = cmd.clone();
    for (k, v) in params {
        let pattern = format!("${{{}}}", k.as_str().unwrap());
        final_cmd = final_cmd.replace(&pattern, v.as_str().unwrap());
    }
    let mut cmd = build_command(&final_cmd);
    execute_command(&mut cmd).await;
}

fn parse_path(mut path: PathBuf) -> PathBuf {
    path = std::fs::canonicalize(path).unwrap();
    if path.is_dir() {
        path.push("dvc.yaml")
    };
    path
}

fn parse_stages(path: &PathBuf) -> Pipeline {
    read_yaml(path)
}

fn read_yaml<T: for<'de> serde::Deserialize<'de>>(path: &PathBuf) -> T {
    match std::fs::File::open(path) {
        Ok(f) => serde_yaml::from_reader(f).unwrap(),
        Err(err) => todo!("{}", err),
    }
}

fn build_command(str_cmd: &str) -> Command {
    let sliced_cmd = split(str_cmd).unwrap();
    let (first, args) = sliced_cmd.split_first().unwrap();
    let mut cmd = Command::new(first);
    cmd.args(args);
    cmd
}

async fn execute_command(cmd: &mut Command) -> i16 {
    // Execute the command
    let child = cmd.spawn();

    // Check if the command was spawned successfully
    match child {
        Ok(mut process) => {
            // Wait for the child process to finish
            let status = process.wait().await;

            // Check the exit status
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        // Script executed successfully
                        0
                    } else {
                        // Script failed
                        eprintln!(
                            "Script failed with exit code: {:?}",
                            exit_status.code()
                        );
                        1
                    }
                },
                Err(e) => {
                    // Handle error waiting for the process
                    eprintln!("Error waiting for process: {}", e);
                    1
                },
            }
        },
        Err(e) => {
            // Handle error spawning the command
            eprintln!("Error spawning command: {}", e);
            1
        },
    }
}
