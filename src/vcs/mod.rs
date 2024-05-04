pub mod cli;
mod comparaison;
mod file;
mod remote;
mod versioning;

pub use cli::VcsArgs;
pub use file::{FileFacade, Remote};
