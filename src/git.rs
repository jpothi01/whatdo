use anyhow::Result;
use std::{
    path::PathBuf,
    process::Output,
    process::{Command, ExitStatus},
};

fn trimmed_stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .trim()
        .to_owned()
}

#[cfg(debug_assertions)]
fn run_command<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Result<Output> {
    let args_vec: Vec<&str> = args.into_iter().collect();
    eprint!("{}", program);
    for arg in &args_vec {
        eprint!(" {}", arg);
    }
    eprint!("");

    let output = Command::new(program).args(args_vec).output()?;
    eprint!("{}", trimmed_stdout(&output));
    eprint!("---");
    Ok(output)
}

#[cfg(not(debug_assertions))]
fn run_command<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Result<Output> {
    Ok(Command::new(program).args(args).output()?)
}

fn simple_command<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Result<String> {
    let output = run_command(program, args)?;
    Ok(trimmed_stdout(&output))
}

pub fn get_root() -> Result<PathBuf> {
    Ok(PathBuf::from(simple_command(
        "git",
        ["rev-parse", "--show-toplevel"],
    )?))
}

pub fn checkout_new_branch(name: &str, push: bool) -> Result<()> {
    simple_command("git", ["checkout", "-b", name])?;
    if push {
        simple_command("git", ["push", "-u", "origin", name])?;
    }

    Ok(())
}

pub fn current_branch() -> Result<String> {
    simple_command("git", ["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn commit(paths: impl IntoIterator<Item = PathBuf>, message: &str, push: bool) -> Result<()> {
    simple_command("git", ["reset"])?;
    for path in paths.into_iter() {
        simple_command("git", ["add", &path.to_string_lossy()])?;
    }
    simple_command("git", ["commit", "-m", message])?;
    if push {
        simple_command("git", ["push"])?;
    }
    Ok(())
}

pub fn default_branch_name() -> Result<String> {
    simple_command("git", ["remote", "set-head", "origin", "-a"])?;
    Ok(String::from_iter(
        simple_command("git", ["rev-parse", "--abbrev-ref", "origin/HEAD"])?
            .chars()
            .skip(7),
    ))
}

pub fn has_unstaged_changes() -> Result<bool> {
    return Ok(simple_command("git", ["status", "--porcelain=v1"])?
        .trim()
        .len()
        > 0);
}

pub fn branch_exists(branch_name: &str) -> Result<bool> {
    let output = run_command("git", ["show-branch", branch_name])?;
    Ok(output.status.success())
}

pub fn merge(target_branch_name: &str, push: bool) -> Result<()> {
    let current_branch_name = current_branch()?;
    simple_command("git", ["checkout", target_branch_name])?;
    simple_command("git", ["merge", &current_branch_name])?;
    if push {
        simple_command("git", ["push"])?;
    }
    Ok(())
}
