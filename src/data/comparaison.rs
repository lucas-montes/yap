use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use meowhash::MeowHasher;
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
    path: Option<PathBuf>,
}

impl Comparaison {
    pub fn new(techniques: &Vec<ComparaisonTechnique>, path: &Option<PathBuf>) -> Self {
        Self {
            techniques: techniques.clone(),
            path: path.to_owned(),
        }
    }

    pub fn compare(&self, file: &FileFacade) -> bool {
        let technique = &self.techniques[0];
        !match technique {
            ComparaisonTechnique::Hash => compare_hash(file),
            ComparaisonTechnique::Custom => compare_custom(
                file,
                self.path.as_ref().expect(
                    "You must provide a script if you select the custom comparaison technique",
                ),
            ),
            ComparaisonTechnique::Similarity => compare_similarity(file),
            ComparaisonTechnique::Smart => compare_smart(file),
        }
    }
}

pub fn compare_smart(file: &FileFacade) -> bool {
    let simple = compare_hash(file);
    match file
        .original_path()
        .extension()
        .and_then(OsStr::to_str)
        .unwrap()
    {
        "md" => compare_similarity(file) & simple,
        "parquet" => compare_similarity(file) & simple,
        _ => simple,
    }
}
pub fn compare_similarity(_file: &FileFacade) -> bool {
    todo!()
}
//TODO: create the functions to compare parquets files with polars, the function to compare text
//only files and the one to call the custom script
pub fn compare_custom(_file: &FileFacade, _script: &Path) -> bool {
    todo!()
}
pub fn compare_hash(file: &FileFacade) -> bool {
    let current_file = fs::read(file.original_path()).expect("cannot open orinigal file");
    let current = MeowHasher::hash(&current_file);
    //TODO: better handl errors
    let previous_file = fs::read(file.previous_version()).expect("cannot open previous copy");
    let previous = MeowHasher::hash(&previous_file);
    current.eq(&previous)
}
