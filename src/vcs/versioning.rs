use std::path::PathBuf;

use crate::config::Author;

use super::{
    comparaison::Diff,
    file::LogbookProvider,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Commit {
    pk: Option<u32>,
    author: Author,
    branch: String,
    diff: Option<Diff>,
    file_from_path: PathBuf,
    file_to_path: PathBuf,
    git_commit: String,
    message: String,
}

impl Commit {
    pub fn new(
        branch: String,
        file_from_path: PathBuf,
        file_to_path: PathBuf,
        message: String,
        author: Author,
    ) -> Self {
        Self {
            pk: None,
            diff: None,
            author,
            branch,
            file_from_path,
            file_to_path,
            message,
            git_commit: String::new(),
        }
    }

    pub fn set_git_commit(mut self, commit: String) -> Self {
        self.git_commit = commit;
        self
    }

    pub fn set_diff(mut self, diff: &Diff) -> Self {
        self.diff = Some(diff.to_owned());
        self
    }

    fn diff_pk(&self) -> String {
        match &self.diff {
            Some(v) => v.pk(),
            None => String::new(),
        }
    }
}

impl LogbookProvider for Commit {
    async fn query(&self) -> String {
        "INSERT INTO commits (git_commit, message, file_from, file_to, diff, branch, author) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)".to_string()
    }
    async fn params(&self) -> Vec<String> {
        //TODO: fix the type return. return the thing of params from libsql
        vec![
            self.git_commit.clone(),
            self.message.clone(),
            self.file_from_path.to_str().unwrap().to_string(),
            self.file_to_path.to_str().unwrap().to_string(),
            self.diff_pk(),
            self.branch.clone(),
            self.author.pk(), //TODO: fix all the structs that should be the ids
        ]
    }
}

pub async fn get_latest_git_commit() -> String {
    match Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .await
    {
        Ok(v) => String::from_utf8(v.stdout).unwrap(),
        Err(_err) => String::new(),
    }
}
