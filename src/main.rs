use core::NextAmount;

use anyhow::{Error, Result};
use clap::{Parser, Subcommand};

use crate::core::{Whatdo, WhatdoTreeView};

extern crate clap;
extern crate colored;
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

        #[arg(long, help = "Automatically start the whatdo")]
        start: bool,

        #[arg(long, help = "Don't commit the change to the git repo, if applicable")]
        no_commit: bool,
    },

    #[command(about = "Show all whatdos or a specific whatdo")]
    Show {
        #[arg(help = "ID of the whatdo to show")]
        id: Option<String>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of tags. Only show whatdos that have one of the given tags"
        )]
        tags: Vec<String>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of priorties. Only show whatdos that have one of the given priorities"
        )]
        priorities: Vec<i64>,
    },

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
            help = "Comma-separated list of tags. Only include whatdos with an ancestor that have one of the given tags"
        )]
        tags: Vec<String>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of priorties. Only include whatdos that have one of the given priorities"
        )]
        priorities: Vec<i64>,
    },

    #[command(about = "Alias for 'show'")]
    Ls {
        #[arg(help = "ID of the whatdo to show")]
        id: Option<String>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of tags. Only show whatdos that have one of the given tags"
        )]
        tags: Vec<String>,

        #[arg(
            short,
            long,
            help = "Comma-separated list of priorties. Only show whatdos that have one of the given priorities"
        )]
        priorities: Vec<i64>,
    },

    #[command(about = "Alias for 'delete'")]
    Rm {
        id: String,

        #[arg(long, help = "Don't commit the change to the git repo, if applicable")]
        no_commit: bool,
    },

    #[command(about = "Delete a whatdo")]
    Delete {
        id: String,

        #[arg(long, help = "Don't commit the change to the git repo, if applicable")]
        no_commit: bool,
    },

    #[command(about = "Mark a whatdo as 'done'. That is, delete it and receive congratulations")]
    Resolve {
        id: String,

        #[arg(long, help = "Don't commit the change to the git repo, if applicable")]
        no_commit: bool,
    },

    #[command(about = "Start a whatdo by checking out a git branch")]
    Start { id: String },

    #[command(
        about = "Finish the current whatdo by resolving it then merging with the parent branch"
    )]
    Finish {
        #[arg(long, help = "Don't commit the whatdo change to the git repo")]
        no_commit: bool,

        #[arg(long, help = "Don't merge to the parent branch after committing")]
        no_merge: bool,
    },

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
    start: bool,
    no_commit: bool,
) -> Result<()> {
    let (new, parent) = core::add(
        &id,
        tags,
        summary.as_ref().map(|s| s.as_str()),
        priority,
        parent,
        !no_commit,
    )?;
    println!("Added:");
    println!("{}", new);

    if let Some(parent) = parent {
        println!("");
        println!("Parent:");
        println!("{}", parent);
    }

    if start {
        core::start(&new)?;
        println!("");
        println!("Started:");
        println!("{}", new);
    }

    Ok(())
}

fn show(id: Option<String>, tags: Vec<String>, priorities: Vec<i64>) -> Result<()> {
    if id.is_some() && (tags.len() > 0 || priorities.len() > 0) {
        return Err(Error::msg(
            "Cannot specify both an ID and tags or priorities",
        ));
    }

    let root = core::root()?;

    if let Some(id) = id {
        let wd = core::get(&id)?;
        match wd {
            None => eprintln!("Not found"),
            Some(_) => {
                print!(
                    "{}",
                    WhatdoTreeView {
                        root,
                        filter: Box::new(move |w| w.id == id),
                        transitive: true
                    }
                )
            }
        }
    } else {
        print!(
            "{}",
            WhatdoTreeView {
                root,
                filter: Box::new(move |w: &Whatdo| {
                    (tags.len() == 0
                        || (w.tags.is_some()
                            && w.tags.as_ref().unwrap().iter().any(|t| tags.contains(t))))
                        && (priorities.len() == 0
                            || (w.priority.is_some() && priorities.contains(&w.priority.unwrap())))
                }),
                transitive: true
            }
        )
    }

    Ok(())
}

fn next(
    start: bool,
    all: bool,
    n: Option<usize>,
    tags: Vec<String>,
    priorities: Vec<i64>,
) -> Result<()> {
    if start && (all || n.filter(|n| n != &1).is_some()) {
        return Err(Error::msg("Cannot specify both --start and --all or -n"));
    }

    let next_amount = if all {
        NextAmount::All
    } else {
        NextAmount::AtMost(n.unwrap_or(1usize))
    };

    let whatdos = core::next(next_amount, tags, priorities)?;
    if start {
        if whatdos.len() == 0 {
            println!("No whatdos to start");
        } else {
            let wd = &whatdos[0];
            core::start(wd)?;
            println!("Started:");
            println!("{}", wd);
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
            println!("Started:");
            println!("{}", wd);
        }
    }
    Ok(())
}

fn finish(no_commit: bool, no_merge: bool) -> Result<()> {
    let wd = core::current()?;
    match wd {
        None => eprintln!("No current whatdo"),
        Some(wd) => {
            core::resolve(&wd.id, !no_commit, !no_merge)?;
            println!("Finished:");
            println!("{}", wd);
            println!("");
            println!("Congratulations!")
        }
    }
    Ok(())
}

fn delete(id: &str, no_commit: bool) -> Result<()> {
    let wd = core::get(id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            core::delete(id, !no_commit)?;
            println!("Deleted:");
            println!("{}", wd);
        }
    }
    Ok(())
}

fn resolve(id: &str, no_commit: bool) -> Result<()> {
    let wd = core::get(id)?;
    match wd {
        None => eprintln!("Not found"),
        Some(wd) => {
            core::resolve(&wd.id, !no_commit, false)?;
            println!("Resolved:");
            println!("{}", wd);
            println!("");
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
            println!("Active:");
            println!("{}", wd);
        }
    }

    println!("");

    let wds = core::next(NextAmount::AtMost(10), vec![], vec![])?;
    if wds.len() > 0 {
        println!("Next few whatdos:");
        for wd in wds {
            println!("{}", wd);
        }
    } else {
        println!("No whatdos coming up. Add some with `wd add`!");
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
            start,
            no_commit,
        }) => add(id, tags, summary, priority, parent, start, no_commit),
        Some(Command::Show {
            id,
            tags,
            priorities,
        }) => show(id, tags, priorities),
        Some(Command::Next {
            start,
            all,
            n,
            tags,
            priorities,
        }) => next(start, all, n, tags, priorities),
        Some(Command::Start { id }) => start(&id),
        Some(Command::Finish {
            no_commit,
            no_merge,
        }) => finish(no_commit, no_merge),
        Some(Command::Delete { id, no_commit }) => delete(&id, no_commit),
        Some(Command::Rm { id, no_commit }) => delete(&id, no_commit),
        Some(Command::Resolve { id, no_commit }) => resolve(&id, no_commit),
        Some(Command::Ls {
            id,
            tags,
            priorities,
        }) => show(id, tags, priorities),
        Some(Command::Status {}) => status(),
        None => status(),
    }
}
