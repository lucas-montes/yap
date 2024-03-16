use menva::get_env;

use clap::ValueEnum;

use super::file::FileFacade;

#[derive(ValueEnum, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum PushStrategy {
    All,
    Last,
    Smart,
}

#[derive(Debug)]
pub enum Protocol {
    Ssh(String),
    Ftp(String),
    Gcs(String),
    Http(String),
}

pub fn push_file(file: &FileFacade) {
    match file.remote() {
        "gcs" => create_operator(),
        _ => todo!(),
    }
}

fn create_operator() -> Operator {
    let mut builder = Gcs::default();
    builder.bucket("test");
    builder.root("/path/to/dir");
    builder.credential_path(get_env("GSC_CREDENTIALS"));
    builder.predefined_acl("publicRead");
    builder.default_storage_class("STANDARD");

    match Operator::new(builder) {
        Ok(op) => op.finish(),
        Err(err) => panic!("{:?}", err),
    }
}
