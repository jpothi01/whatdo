use clap::{Parser, Subcommand};

extern crate clap;
extern crate serde_yaml;
extern crate yaml_rust;

mod core;

#[derive(Subcommand, Debug, Clone)]
enum Command {
    // Add a new whatdo
    #[command(about = "Add a new whatdo")]
    Add {
        id: String,

        #[arg(short, long)]
        tags: Vec<String>,
    },
    #[command(about = "Show a whatdo")]
    Show { id: String },
    #[command(about = "Start a whatdo by checking out a git branch")]
    Start { id: String },
    // /// list all the projects
    // Projects {
    //     #[clap(short, long, default_value_t = String::from("."),forbid_empty_values = true, validator = validate_package_name)]
    //     /// directory to start exploring from
    //     start_path: String,
    //     #[clap(short, long, multiple_values = true)]
    //     /// paths to exclude when searching
    //     exclude: Vec<String>,
    // },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    cmd: Option<Command>,
}

fn main() {
    let args = Args::parse();

    match args.cmd {
        Some(Command::Add { id, tags }) => core::add(&id),
        None => core::list(),
        _ => {}
    }
}
