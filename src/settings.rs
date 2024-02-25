use std::fs::File;
use std::io::prelude::*;

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

#[derive(Debug, Deserialize)]
pub struct Settings {}
impl Settings {
    pub fn new() -> Self {
        Self {}
    }
}

fn read_toml() {
    // Specify the path to your TOML file
    let file_path = ".dvc";

    // Read the contents of the file into a String
    let mut file = File::open(file_path).expect("Unable to open file");
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)
        .expect("Unable to read file");

    // Parse the TOML data into the Config struct
    let config: DvcConfig = toml::from_str(&toml_string).expect("Unable to parse TOML");

    // Access values in the Config struct
    println!("Core remote: {}", config.core.remote);

    // Extract remote configurations
    for (remote_name, remote_config) in config.remotes.iter() {
        if let Some(remote) = remote_config.get("url").and_then(|url| url.as_str()) {
            println!("Remote {} URL: {}", remote_name, remote);
        }
    }
}
