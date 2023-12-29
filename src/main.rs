use anyhow::Result;
use clap::{Parser, Subcommand};

extern crate clap;
extern crate serde_yaml;
extern crate sqlite;
extern crate yaml_rust;

mod core;
mod git;

#[derive(Subcommand, Debug, Clone)]
enum Command {
    // Add a new whatdo
    #[command(about = "Add a new whatdo")]
    Add {
        id: String,

        #[arg(short, long)]
        tags: Vec<String>,

        #[arg(short = 'm', long)]
        summary: Option<String>,

        #[arg(short, long)]
        priority: Option<i64>,
    },
    #[command(about = "Show a whatdo")]
    Show { id: String },
    #[command(about = "Show the next whatdo in the queue")]
    Next {
        #[clap(long, help = "Automatically start the whatdo")]
        start: bool,
    },

    #[command(about = "Alias for 'status'")]
    Ls {},
    #[command(about = "Alias for 'delete'")]
    Rm { id: String },
    #[command(about = "Delete a whatdo")]
    Delete { id: String },
    #[command(about = "Mark a whatdo as 'done'. That is, delete it and receive congratulations")]
    Resolve { id: String },

    #[command(about = "Start a whatdo by checking out a git branch")]
    Start { id: String },
    #[command(
        about = "Finish the current whatdo by resolving it and committing to the active branch"
    )]
    Finish {},

    #[command(about = "Display the active whatdo and the next few to do")]
    Status {},
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    cmd: Option<Command>,
}

fn add(
    id: String,
    tags: Vec<String>,
    summary: Option<String>,
    priority: Option<i64>,
) -> Result<()> {
    core::add(&id, tags, summary.as_ref().map(|s| s.as_str()), priority)?;
    Ok(())
}

fn show(id: String) -> Result<()> {
    let wd = core::get(&id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            println!("{}", wd.detailed_display())
        }
    }
    Ok(())
}

fn next(start: bool) -> Result<()> {
    let next = core::next()?;
    match next {
        None => println!(""),
        Some(wd) => {
            if start {
                core::start(&wd)?;
                println!("Starting {}", wd)
            } else {
                println!("{}", wd)
            }
        }
    }
    Ok(())
}

fn start(id: &str) -> Result<()> {
    let wd = core::get(id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            core::start(&wd)?;
            println!("Starting {}", wd)
        }
    }
    Ok(())
}

fn finish() -> Result<()> {
    let wd = core::current()?;
    match wd {
        None => eprintln!("No current whatdo"),
        Some(wd) => {
            core::delete(&wd.id)?;
            println!("Finished {}", wd);
            println!("Congratulations!")
        }
    }
    Ok(())
}

fn delete(id: &str) -> Result<()> {
    let wd = core::get(id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            core::delete(id)?;
            println!("Deleted {}", &wd)
        }
    }
    Ok(())
}

fn resolve(id: &str) -> Result<()> {
    let wd = core::get(id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            core::delete(id)?;
            println!("Deleted {}", &wd);
            println!("Well done!");
        }
    }
    Ok(())
}

fn status() -> Result<()> {
    let wd = core::current()?;
    match wd {
        None => println!("No active whatdo"),
        Some(wd) => {
            println!("Active: {}", wd);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Some(Command::Add {
            id,
            tags,
            summary,
            priority,
        }) => add(id, tags, summary, priority),
        Some(Command::Show { id }) => show(id),
        Some(Command::Next { start }) => next(start),
        Some(Command::Start { id }) => start(&id),
        Some(Command::Finish {}) => finish(),
        Some(Command::Delete { id }) => delete(&id),
        Some(Command::Rm { id }) => delete(&id),
        Some(Command::Resolve { id }) => resolve(&id),
        Some(Command::Ls {}) => status(),
        Some(Command::Status {}) => status(),
        None => status(),
        _ => Ok(()),
    }
}
