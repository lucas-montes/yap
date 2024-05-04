use std::{
    ffi::OsStr,
    fmt, fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use meowhash::MeowHasher;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

impl fmt::Display for ComparaisonTechnique {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Diff {
    pk: Option<u32>,
    git_commit: Option<String>,
    result_path: PathBuf,
    script: Option<PathBuf>,
    result: Value,
    file_from: File,
    file_to: File,
    branch: String,
    author: Author,
    technique: ComparaisonTechnique,
}

impl Diff {
    pub fn new(script: Option<PathBuf>, file_from: File, file_to: File) -> Self {
        Self {
            script,
            file_from,
            file_to,
            ..Self::default()
        }
    }

    fn set_result(mut self, result: Value) -> Self {
        self.result = result;
        self
    }

    pub fn pk(&self) -> String {
        match self.pk {
            Some(v) => v.to_string(),
            None => String::new(),
        }
    }

    fn script(&self) -> String {
        match &self.script {
            Some(v) => v.to_str().unwrap().to_owned(),
            None => String::new(),
        }
    }

    pub fn merge_results(a: &mut Value, b: &Value) {
        match (a, b) {
            (Value::Object(a), Value::Object(b)) => {
                for (k, v) in b {
                    Self::merge_results(a.entry(k.clone()).or_insert(Value::Null), v);
                }
            },
            (a, b) => *a = b.clone(),
        }
    }
}

impl LogbookProvider for Diff {
    //TODO: some values are missing add them
    async fn query(&self) -> String {
        "INSERT INTO diffs (git_commit, result_path, script, result, technique, file_from, file_to, author, branch) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)".to_string()
    }
    async fn params(&self) -> Vec<String> {
        vec![
            self.git_commit
                .as_ref()
                .unwrap_or(&String::new())
                .to_string(),
            self.result_path.to_str().unwrap().to_string(),
            self.script(),
            self.result.to_string(),
            self.technique.to_string(),
            self.file_from.original_path().to_str().unwrap().to_string(),
            self.file_to.original_path().to_str().unwrap().to_string(),
            self.author.to_string(),
            self.branch.clone(),
        ]
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

    pub fn result(&self) -> Diff {
        self.result.as_ref().unwrap().clone()
    }

    pub fn compare(&mut self, current: &FileFacade, previous: &FileFacade) -> &Self {
        let diff_result = match self.technique {
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
        let result = Diff::new(self.path.clone(), previous.file(), current.file())
            .set_result(diff_result);
        self.result = Some(result);
        self
    }

    pub fn compare_smart(&self, current: &FileFacade, previous: &FileFacade) -> Value {
        let mut first = self.compare_hash(current, previous);
        let second = match current
            .original_path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap()
        {
            "md" => self.compare_similarity(current, previous),
            "parquet" => self.compare_similarity(current, previous),
            _ => first.clone(),
        };
        Diff::merge_results(&mut first, &second);
        first
    }

    pub fn compare_similarity(
        &self,
        current: &FileFacade,
        previous: &FileFacade,
    ) -> Value {
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
    ) -> Value {
        todo!()
    }
    pub fn compare_hash(&self, current: &FileFacade, previous: &FileFacade) -> Value {
        let current_file =
            fs::read(current.original_path()).expect("cannot open orinigal file");
        let current = MeowHasher::hash(&current_file);
        //TODO: better handl errors
        let previous_file =
            fs::read(previous.original_path()).expect("cannot open previous copy");
        let previous = MeowHasher::hash(&previous_file);
        let _ = current.eq(&previous);
        json!({"":""})
    }
}
