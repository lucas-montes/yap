use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use toml::de::Error;
use toml::value::Table;
use toml::Value;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Core {
    remote: String,
}

#[derive(Debug, Deserialize)]
struct RemoteConfig {
    url: String,
}

#[derive(Debug, Deserialize)]
struct DvcConfig {
    core: Core,

    #[serde(flatten)]
    remotes: Table,
}

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    //Turso settings for the databases
    pub organization: String,
    pub local: PathBuf,
    pub remote: String,
    pub group: String,
    #[serde(default)]
    pub remote_url: String,
}

impl Settings {
    pub fn new() -> Self {
        toml_to_struct(".yap/.config")
    }
}

fn toml_to_struct<T: for<'a> Deserialize<'a>>(path: &str) -> T {
    let mut file = File::open(path).expect("Unable to open file");
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)
        .expect("Unable to read file");
    toml::from_str(&toml_string).expect("Unable to parse TOML")
}

fn read_toml() {
    let config: DvcConfig = toml_to_struct(".dvc");
    //for (remote_name, remote_config) in config.remotes.iter() {
    //    if let Some(remote) = remote_config.get("url").and_then(|url| url.as_str()) {
    //        println!("Remote {} URL: {}", remote_name, remote);
    //    }
    //}
}
