use clap::{Parser, Subcommand};

extern crate clap;

mod core;

#[derive(Subcommand, Debug, Clone)]
enum Command {
    Add { id: String },
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
struct Args {
    #[clap(subcommand)]
    cmd: Option<Command>,
}

fn main() {
    let args = Args::parse();

    match args.cmd {
        Some(Command::Add { id }) => core::add(&id),
        None => core::list(),
    }
}
