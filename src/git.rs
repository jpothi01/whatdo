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

pub fn checkout_new_branch(name: &str) -> Result<()> {
    Command::new("git")
        .args(["checkout", "-b", name])
        .output()?;
    Ok(())
}

pub fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
}

pub fn commit(paths: impl IntoIterator<Item = PathBuf>, message: &str) -> Result<()> {
    Command::new("git").args(["reset"]).output()?;
    for path in paths.into_iter() {
        Command::new("git")
            .args(["add", &path.to_string_lossy()])
            .output()?;
    }
    Command::new("git")
        .args(["commit", "-m", message])
        .output()?;
    Ok(())
}
