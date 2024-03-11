use crate::{
    config::Config,
    versioning::{Events, FileFacade},
};

pub async fn compare(file: &FileFacade, config: &Config) {
    file.create_snapshot(config, &Events::Commit).await;
}
