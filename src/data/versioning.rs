use super::{comparaison::Diff, file::{File, LogbookProvider}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Default, Serialize)]
pub struct Commit {
    //author:Author,
    branch: String,
    diff: Option<Diff>,
    file_from: File,
    file_to: File,
    git_commit: Option<String>,
    message: String,
}

impl Commit {
    pub fn new(branch: String, file_from: File, file_to: File, message: String) -> Self {
        Self {
            branch,
            file_to,
            file_from,
            message,
            ..Self::default()
        }
    }
    pub fn set_diff(mut self, diff: &Diff)->Self{
        self.diff = Some(diff.to_owned());
        self
    }
}

impl LogbookProvider for Commit{
    fn query(&self) -> String {
        todo!()
    }
    fn params(&self) -> Vec<String> {
        todo!()
    }
}
