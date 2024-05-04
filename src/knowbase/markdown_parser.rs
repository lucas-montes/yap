use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::PathBuf;
use tokio::{
    fs::{canonicalize, File},
    io::AsyncReadExt,
};

use super::todo::TaskFactory;
#[allow(dead_code)]
pub async fn parse_markdown(paths: &Vec<PathBuf>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    for path in paths {
        parse_file(path, options).await;
    }
}
#[allow(dead_code)]
async fn parse_file(path: &PathBuf, parser_options: Options) {
    let mut f = match File::open(path).await {
        Ok(f) => f,
        Err(err) => panic!("parse_file markdown error: {:?}", err),
    };
    let mut markdown_content = String::new();
    match f.read_to_string(&mut markdown_content).await {
        Ok(_) => {},
        Err(err) => panic!("parse_file to string error: {:?}", err),
    };
    read_events(path, &markdown_content, parser_options).await;
}
#[allow(dead_code)]
async fn read_events(path: &PathBuf, markdown_content: &str, parser_options: Options) {
    let parser = Parser::new_ext(markdown_content, parser_options);
    let mut is_list = false;
    let mut tasks = TaskFactory::new();
    let abs_path = match canonicalize(path).await {
        Ok(a) => a.to_str().unwrap().to_string(),
        Err(err) => panic!("{err}"),
    };
    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Link {
                    link_type: _,
                    dest_url: _,
                    title: _,
                    id: _,
                } => {},
                Tag::List(_) => {
                    is_list = true;
                },
                _ => {},
            },
            Event::Text(text) => {
                if !text.is_empty() && is_list {
                    tasks
                        .set_description(abs_path.clone())
                        .set_title(text.to_string())
                        .append();
                }
            },
            Event::TaskListMarker(completed) => {
                tasks.set_done(completed);
            },
            Event::End(tag) => match tag {
                TagEnd::List(_) => {
                    is_list = false;
                },
                TagEnd::Link => {},
                TagEnd::Paragraph => {},
                _ => {},
            },
            _ => {},
        }
    }
    tasks.save();
}
