use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir, OpenOptions},
    io::prelude::*,
    path::Path,
};

pub fn toml_to_struct<T: for<'a> Deserialize<'a>>(path: &str) -> T {
    //TODO: use tokio
    let parent = Path::new(path).parent().unwrap();
    if !parent.exists() {
        create_dir(parent).unwrap();
    }
    let mut toml_string = String::new();
    let mut file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(value) => value,
        Err(e) => panic!("There is no file: {:?}", e),
    };
    file.read_to_string(&mut toml_string)
        .expect("Unable to read file");
    toml::from_str(&toml_string).expect("Unable to parse TOML. This might be caused by a missing quote in a string value for example.")
}

pub fn struct_to_toml<T: Serialize>(instance: &T, path: &str) {
    let data = toml::to_string_pretty(instance).expect("Unable to parse TOML");
    let mut file = match OpenOptions::new().write(true).truncate(true).open(path) {
        Ok(value) => value,
        Err(e) => panic!("Problem creating the file: {:?}", e),
    };
    let _ = file.write_all(data.as_bytes());
}
