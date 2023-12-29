use anyhow::Result;
use std::{path::PathBuf, process::Command};

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
        .args(["rev-parse", "abbrev-ref", "HEAD"])
        .output()?;
    Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
}
