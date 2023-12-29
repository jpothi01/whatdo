use core::NextAmount;

use anyhow::{Error, Result};
use clap::{Parser, Subcommand};

extern crate clap;
extern crate env_logger;
extern crate log;
extern crate once_cell;
extern crate regex;
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
        #[arg(
            help = "Short, unique, slug-style identifier for the whatdo. This will be used as the branch name if using `wd start`"
        )]
        id: String,

        #[arg(
            short,
            long,
            help = "Comma-separated list of slug-style strings used to group and filter whatdos"
        )]
        tags: Vec<String>,

        #[arg(
            short = 'm',
            long,
            help = "Freetext description of what this whatdo is about"
        )]
        summary: Option<String>,

        #[arg(
            short,
            long,
            help = "Integer priority of the whatdo. Whatdos with lower values for priority are selected *first*"
        )]
        priority: Option<i64>,

        #[arg(long, help = "ID of the parent whatdo, if any")]
        parent: Option<String>,
    },
    #[command(about = "Show a whatdo")]
    Show { id: String },
    #[command(about = "Show the next whatdo in the queue")]
    Next {
        #[clap(
            long,
            help = "Automatically start the whatdo. Incompatible with --all and -n"
        )]
        start: bool,

        #[clap(long, help = "Show all next whatdos")]
        all: bool,

        #[clap(short = 'n', help = "Number of next whatdos to show")]
        n: Option<usize>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of tags. Only include whatdos with an ancestor that has one of the given tags"
        )]
        tags: Vec<String>,
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
    parent: Option<String>,
) -> Result<()> {
    core::add(
        &id,
        tags,
        summary.as_ref().map(|s| s.as_str()),
        priority,
        parent,
    )?;
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

fn next(start: bool, all: bool, n: Option<usize>, tags: Vec<String>) -> Result<()> {
    if start && (all || n.filter(|n| n != &1).is_some()) {
        return Err(Error::msg("Cannot specify both --start and --all or -n"));
    }

    let next_amount = if all {
        NextAmount::All
    } else {
        NextAmount::AtMost(n.unwrap_or(1usize))
    };

    let whatdos = core::next(next_amount, tags)?;
    if start {
        if whatdos.len() == 0 {
            println!("No whatdos to start");
        } else {
            let wd = &whatdos[0];
            core::start(wd)?;
            println!("Starting {}", wd)
        }
    } else {
        for wd in whatdos {
            println!("{}", wd);
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
    env_logger::init();
    let args = Args::parse();

    match args.cmd {
        Some(Command::Add {
            id,
            tags,
            summary,
            priority,
            parent,
        }) => add(id, tags, summary, priority, parent),
        Some(Command::Show { id }) => show(id),
        Some(Command::Next {
            start,
            all,
            n,
            tags,
        }) => next(start, all, n, tags),
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
