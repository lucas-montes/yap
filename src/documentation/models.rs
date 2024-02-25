
use crate::enums::ColorWhen;

use clap::{Args, Subcommand};

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

#[derive(Debug, Subcommand)]
pub enum DocsCommands {
    /// Create a new object
    #[command(arg_required_else_help = true)]
    Add(AddDocs),

    /// Update an actual object
    #[command(arg_required_else_help = true)]
    Push(UpdateDocs),

    /// Delete one object
    #[command(arg_required_else_help = true)]
    Delete(DeleteDocs),

    /// remove one object
    #[command(arg_required_else_help = true)]
    Remove(RemoveDocs),

    /// Read one or more objects
    #[command(arg_required_else_help = true)]
    Get(GetDocs),
}

impl DocsCommands {
    pub fn handle_commands(&self) -> i16 {
        match self {
            DocsCommands::Add(args) => args.run(),
            DocsCommands::Push(args) => args.run(),
            DocsCommands::Delete(args) => args.run(),
            DocsCommands::Get(args) => args.run(),
            DocsCommands::Remove(args) => args.run(),
        };
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct AddDocs {
    #[arg(short, long)]
    title: String,
}

impl AddDocs {
    fn run(&self) -> i16 {
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct DeleteDocs {
    #[arg(short, long)]
    id: Option<i16>,
    #[arg(short, long)]
    title: Option<String>,
}

impl DeleteDocs {
    fn run(&self) -> i16 {
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct RemoveDocs {
    #[arg(short, long)]
    id: Option<i16>,
    #[arg(short, long)]
    title: Option<String>,
}

impl RemoveDocs {
    fn run(&self) -> i16 {
        0
    }
}
#[derive(Debug, Args, Clone)]
pub struct GetDocs {
    #[arg(short, long, default_value_t = true, required=false)]
    all: bool,
}

impl GetDocs {
    fn run(&self) -> i16 {
        0
    }
}

#[derive(Debug, Args, Clone)]
pub struct UpdateDocs {
    #[arg(short, long)]
    id: i16,
}

impl UpdateDocs {
    fn run(&self) -> i16 {
        0
    }
}

