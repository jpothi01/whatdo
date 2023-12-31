use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn get_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    return Ok(PathBuf::from(
        &String::from_utf8(output.stdout).unwrap().trim(),
    ));
}

pub fn checkout_new_branch(name: &str, push: bool) -> Result<()> {
    Command::new("git")
        .args(["checkout", "-b", name])
        .output()?;
    if push {
        Command::new("git")
            .args(["push", "-u", "origin", name])
            .output()?;
    }

    Ok(())
}

pub fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
}

pub fn commit(paths: impl IntoIterator<Item = PathBuf>, message: &str, push: bool) -> Result<()> {
    Command::new("git").args(["reset"]).output()?;
    for path in paths.into_iter() {
        Command::new("git")
            .args(["add", &path.to_string_lossy()])
            .output()?;
    }
    Command::new("git")
        .args(["commit", "-m", message])
        .output()?;
    if push {
        Command::new("git").args(["push"]).output()?;
    }
    Ok(())
}

fn default_branch_name() -> Result<String> {
    Command::new("git")
        .args(["remote", "set-head", "origin", "-a"])
        .output()?;
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "origin/HEAD"])
        .output()?;
    return Ok(String::from_utf8(output.stdout).unwrap().trim().into());
}

pub fn merge(target_branch: Option<&str>, push: bool) -> Result<()> {
    let target_branch_name = if let Some(branch) = target_branch {
        branch.to_owned()
    } else {
        default_branch_name()?
    };
    let current_branch_name = current_branch()?;
    Command::new("git")
        .args(["checkout", &target_branch_name])
        .output()?;
    Command::new("git")
        .args(["merge", &current_branch_name])
        .output()?;
    if push {
        Command::new("git").args(["push"]).output()?;
    }
    Ok(())
}

pub fn push() -> Result<()> {
    Command::new("git").args(["push"]).output()?;
    Ok(())
}
