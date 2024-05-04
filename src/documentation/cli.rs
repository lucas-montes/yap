use std::{
    collections::HashSet, fs::read_dir, path::PathBuf, str::FromStr, time::Duration,
};

use crate::{
    enums::ColorWhen,
    utils::{struct_to_toml, toml_to_struct},
};

use clap::{Args, Subcommand};
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct DocsArgs {
    #[command(subcommand)]
    pub command: DocsCommands,
    #[arg(
        long,
        require_equals = true,
        value_name = "WHEN",
        num_args = 0..=1,
        default_value_t = ColorWhen::Auto,
        default_missing_value = "always",
        value_enum
    )]
    color: ColorWhen,
}

// Manage documentation. This documentation could be for a software project or even regular
// notes that you could have.
#[derive(Debug, Subcommand)]
pub enum DocsCommands {
    /// Read one or more objects
    #[command(arg_required_else_help = true)]
    Get(GetDocs),
}

impl DocsCommands {
    pub fn handle_commands(&self) -> i16 {
        match self {
            DocsCommands::Get(args) => args.run(),
        };
        0
    }
}
#[derive(Debug, Args, Clone)]
pub struct GetDocs {
    // A list of paths or urls to projects (git only currently). Based on the protocol if any it
    // will clone them and then keep only the documentation.
    #[arg(short, long, required = true)]
    projects: Vec<String>,

    // Relate the projects in a common group. This will allow to search and find relationships
    // between them
    #[arg(short, long, default_value = "master")]
    group: String,

    #[arg(short, long, default_value = "master")]
    branch: String,
}

impl GetDocs {
    fn run(&self) -> i16 {
        let mut docs = Docs::new();
        let tracked: HashSet<String> =
            HashSet::from_iter(docs.projects.iter().map(|p| p.name.clone()));
        let mut new_projects: Vec<DocProject> = self
            .projects
            .iter()
            .filter(|p| !tracked.contains(p.to_owned()))
            .map(|p| self.get_project(p))
            .collect();
        docs.projects.append(&mut new_projects);
        docs.save();
        0
    }

    fn get_project(&self, project: &str) -> DocProject {
        DocProject::new(self.group.clone(), self.branch.clone()).set(project)
    }
}

//struct DocProjectInterface {
//    project: DocProject,
//    progress: ProgressBar,
//}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DocProject {
    #[serde(default)]
    path: PathBuf,
    #[serde(default)]
    url: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    branch: String,
    #[serde(default)]
    group: String,
}

impl DocProject {
    fn new(group: String, branch: String) -> Self {
        Self {
            group,
            branch,
            ..Self::default()
        }
    }

    fn set(mut self, project: &str) -> Self {
        self.name = project
            .split('/')
            .last()
            .unwrap()
            .trim_end_matches(".git")
            .to_string();
        if project.starts_with("http://")
            || project.starts_with("https://")
            || project.starts_with("git@github.com:")
        {
            self.url = project.to_string();
            self.handle_git()
        } else {
            self.path = PathBuf::from(project);
            self.handle_local()
        }
    }

    fn handle_git(mut self) -> Self {
        let mut path = Docs::get_docs_path();
        path.push(&self.group);
        path.push(&self.name);
        println!("{:?}", &self);

        let style = ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner} Cloning... {wide_msg}")
            .unwrap();
        let mut ssh_keys = PathBuf::from_str(&menva::get_env("HOME")).unwrap();
        ssh_keys.push(".ssh");
        let mut keys = HashSet::new();
        for identity in read_dir(ssh_keys).unwrap() {
            // Create a progress bar
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::new(0, 300000000));
            pb.set_style(style.clone());
            let path = identity.unwrap();
            pb.set_message(format!("{:?}", &path));
            keys.insert(path.file_name());
        }
        //keys.for_each(|k| self.pull(&path, k));
        //TODO: uncomment panic!("");
        //let _repo = match Repository::clone(self.url.as_ref(), &path) {
        //    Ok(repo) => repo,
        //    Err(e) => panic!("failed to clone: {}", e),
        //};
        self.path = path;
        self
    }

    //fn pull(&self, path: &Path) {
    //    // Prepare callbacks.
    //    let mut callbacks = RemoteCallbacks::new();
    //    callbacks.credentials(|_url, username_from_url, _allowed_types| {
    //        Cred::ssh_key(
    //            username_from_url.unwrap(),
    //            None,
    //            Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
    //            None,
    //        )
    //    });
    //    // Prepare fetch options.
    //    let mut fo = git2::FetchOptions::new();
    //    fo.remote_callbacks(callbacks);

    //    // Prepare builder.
    //    let mut builder = git2::build::RepoBuilder::new();
    //    builder.fetch_options(fo);

    //    // Clone the project.
    //    let _ = match builder.clone(self.url.as_ref(), path) {
    //        Ok(repo) => repo,
    //        Err(e) => panic!("failed to clone from builder: {}", e),
    //    };
    //}

    fn handle_local(mut self) -> Self {
        let repo = Repository::open(&self.path).unwrap();
        let master = repo
            .remotes()
            .unwrap()
            .iter()
            .map(|r| repo.find_remote(r.unwrap()).unwrap())
            .filter(|r| r.name().is_some_and(|n| n.eq("origin")))
            .last()
            .unwrap();
        self.url = master.url().unwrap().to_owned();
        self
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Docs {
    #[serde(default)]
    projects: Vec<DocProject>,
}
impl Docs {
    fn get_index_path() -> PathBuf {
        let mut home = get_absolute();
        home.push(".yap/docs.toml");
        home
    }
    fn get_docs_path() -> PathBuf {
        let mut home = get_absolute();
        home.push(".yap/docs");
        home
    }
    fn new() -> Self {
        //TODO: we should create this in the configs
        let home = Self::get_index_path();
        toml_to_struct(home.to_str().unwrap())
    }
    fn save(self) {
        let home = Self::get_index_path();
        struct_to_toml(&self, home.to_str().unwrap())
    }
}
fn get_absolute() -> PathBuf {
    let home = menva::get_env("HOME");
    PathBuf::from_str(&home).unwrap()
}
