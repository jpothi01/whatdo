use anyhow::Result;
use std::{path::PathBuf, process::Command};

#[cfg(debug_assertions)]
fn run_command<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Result<String> {
    let args_vec: Vec<&str> = args.into_iter().collect();
    print!("{}", program);
    for arg in &args_vec {
        print!(" {}", arg);
    }
    println!();

    let output = Command::new(program).args(args_vec).output()?;
    let s = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    println!("{}", s);
    println!("---");
    Ok(s)
}

#[cfg(not(debug_assertions))]
fn run_command<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    let s = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    Ok(s)
}

pub fn get_root() -> Result<PathBuf> {
    Ok(PathBuf::from(run_command(
        "git",
        ["rev-parse", "--show-toplevel"],
    )?))
}

pub fn checkout_new_branch(name: &str, push: bool) -> Result<()> {
    run_command("git", ["checkout", "-b", name])?;
    if push {
        run_command("git", ["push", "-u", "origin", name])?;
    }

    Ok(())
}

pub fn current_branch() -> Result<String> {
    run_command("git", ["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn commit(paths: impl IntoIterator<Item = PathBuf>, message: &str, push: bool) -> Result<()> {
    run_command("git", ["reset"])?;
    for path in paths.into_iter() {
        run_command("git", ["add", &path.to_string_lossy()])?;
    }
    run_command("git", ["commit", "-m", message])?;
    if push {
        run_command("git", ["push"])?;
    }
    Ok(())
}

fn default_branch_name() -> Result<String> {
    run_command("git", ["remote", "set-head", "origin", "-a"])?;
    Ok(String::from_iter(
        run_command("git", ["rev-parse", "--abbrev-ref", "origin/HEAD"])?
            .chars()
            .skip(7),
    ))
}

pub fn has_unstaged_changes() -> Result<bool> {
    return Ok(run_command("git", ["status", "--porcelain=v1"])?
        .trim()
        .len()
        > 0);
}

pub fn merge(target_branch: Option<&str>, push: bool) -> Result<()> {
    let target_branch_name = if let Some(branch) = target_branch {
        branch.to_owned()
    } else {
        default_branch_name()?
    };
    let current_branch_name = current_branch()?;
    run_command("git", ["checkout", &target_branch_name])?;
    run_command("git", ["merge", &current_branch_name])?;
    if push {
        run_command("git", ["push"])?;
    }
    Ok(())
}
