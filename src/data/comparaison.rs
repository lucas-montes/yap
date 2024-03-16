use std::{
    ffi::{OsStr, OsString},
    fs,
    path::PathBuf,
};

use clap::ValueEnum;
use meowhash::{MeowHash, MeowHasher};
use serde::{Deserialize, Serialize};

use super::file::FileFacade;

#[derive(ValueEnum, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum ComparaisonTechnique {
    Hash,
    Custom,
    Similarity,
    Smart,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Comparaison {
    #[serde(default)]
    techniques: Vec<ComparaisonTechnique>,
    #[serde(default)]
    path: String,
}

pub fn compare_smart(file: &FileFacade) -> bool {
    match file
        .original_path()
        .extension()
        .and_then(OsStr::to_str)
        .unwrap()
    {
        //"md" => compare_similarity(&file),
        //"parquet" => compare_similarity(&file),
        _ => compare_hash(&file),
    }
}
pub fn compare_similarity(file: &FileFacade) -> bool {
    todo!()
}
//TODO: create the functions to compare parquets files with polars, the function to compare text
//only files and the one to call the custom script
pub fn compare_custom(file: &FileFacade, script: &PathBuf) -> bool {
    todo!()
}
pub fn compare_hash(file: &FileFacade) -> bool {
    let current_file = fs::read(&file.original_path()).expect("cannot open orinigal file");
    let current = MeowHasher::hash(&current_file);
    //TODO: better handl errors
    let previous_file = fs::read(&file.previous_version()).expect("cannot open previous copy");
    let previous = MeowHasher::hash(&previous_file);
    current.eq(&previous)
}
