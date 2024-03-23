use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use meowhash::MeowHasher;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::Author;

use super::file::{File, FileFacade, LogbookProvider};

#[derive(ValueEnum, Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum ComparaisonTechnique {
    Hash,
    Custom,
    Similarity,
    #[default]
    Smart,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Diff {
    git_commit: Option<String>,
    result_path: PathBuf,
    script: PathBuf,
    result: Value,
    file_from: File,
    file_to: File,
    branch: String,
    author: Author,
    techniques: Vec<ComparaisonTechnique>,
}
impl LogbookProvider for Diff{
    fn query(&self) -> String {
        todo!()
    }

    fn params(&self) -> Vec<String> {
        todo!()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Comparaison {
    #[serde(default)]
    technique: ComparaisonTechnique,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    result: Option<Diff>,
}

impl Comparaison {
    pub fn new(technique: &ComparaisonTechnique, path: &Option<PathBuf>) -> Self {
        Self {
            technique: technique.to_owned(),
            path: path.to_owned(),
            result: None,
        }
    }

    pub fn result(&self) -> &Diff {
        self.result.as_ref().unwrap()
    }

    pub fn compare(&self, current: &FileFacade, previous: &FileFacade) -> &Self {
        match self.technique {
            ComparaisonTechnique::Hash => self.compare_hash(current, previous),
            ComparaisonTechnique::Custom => self.compare_custom(
                current,
                previous,
                self.path.as_ref().expect(
                    "You must provide a script if you select the custom comparaison technique",
                ),
            ),
            ComparaisonTechnique::Similarity => self.compare_similarity(current, previous),
            ComparaisonTechnique::Smart => self.compare_smart(current, previous),
        };
        self
    }

    pub fn compare_smart(&self, current: &FileFacade, previous: &FileFacade) -> bool {
        let simple = self.compare_hash(current, previous);
        match current
            .original_path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap()
        {
            "md" => self.compare_similarity(current, previous) & simple,
            "parquet" => self.compare_similarity(current, previous) & simple,
            _ => simple,
        }
    }
    pub fn compare_similarity(&self, current: &FileFacade, previous: &FileFacade) -> bool {
        //TODO: rm this awesome trick
        self.compare_hash(current, previous)
    }
    //TODO: create the functions to compare parquets files with polars, the function to compare text
    //only files and the one to call the custom script
    pub fn compare_custom(
        &self,
        _current: &FileFacade,
        _previous: &FileFacade,
        _script: &Path,
    ) -> bool {
        todo!()
    }
    pub fn compare_hash(&self, current: &FileFacade, previous: &FileFacade) -> bool {
        let current_file = fs::read(current.original_path()).expect("cannot open orinigal file");
        let current = MeowHasher::hash(&current_file);
        //TODO: better handl errors
        let previous_file = fs::read(previous.original_path()).expect("cannot open previous copy");
        let previous = MeowHasher::hash(&previous_file);
        current.eq(&previous)
    }
}
