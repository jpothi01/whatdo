use anyhow::{Error, Result};
use serde_derive::{Deserialize, Serialize};
use std::path::{Component, Path};
use std::process::Command;
use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};
use yaml_rust::yaml;

type ParsedWhatdoMap = HashMap<String, ParsedWhatdo>;

#[derive(Serialize, Deserialize)]
struct ParsedWhatdo {
    whatdos: Option<ParsedWhatdoMap>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Whatdo {
    id: String,
    summary: String,
    whatdos: Vec<Whatdo>,
}

impl Whatdo {
    pub fn simple(id: String, summary: String) -> Self {
        Whatdo {
            id,
            summary,
            whatdos: vec![],
        }
    }
}

fn get_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    return Ok(PathBuf::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap());
}

fn get_project_name(path: &Path) -> Result<String> {
    match path.components().nth_back(1) {
        Some(Component::Normal(n)) => Ok(n.to_str().unwrap().to_owned()),
        _ => Err(Error::msg("Cannot use wd from the root of the filesystem")),
    }
}

fn parse_whatdo(id: &str, data: &serde_yaml::Value) -> Result<Whatdo> {
    match data {
        serde_yaml::Value::String(s) => Ok(Whatdo::simple(id.to_owned(), s.clone())),
        serde_yaml::Value::Mapping(items) => {
            let summary = match items.get("summary") {
                None => return Err(Error::msg("Expected 'summary' key")),
                Some(s) => match s {
                    serde_yaml::Value::String(s) => s,
                    _ => return Err(Error::msg("Expected 'summary' to be a string")),
                },
            };
            let whatdos_map = match items.get("whatdos") {
                None => serde_yaml::Mapping::new(),
                Some(d) => match d {
                    serde_yaml::Value::Mapping(d) => d.clone(),
                    _ => return Err(Error::msg("Expected 'whatdos' to be a mapping")),
                },
            };
            let whatdos: Result<Vec<Whatdo>> = whatdos_map
                .iter()
                .map(|(k, v)| {
                    let id = match k {
                        serde_yaml::Value::String(s) => s,
                        _ => return Err(Error::msg("Expected mapping key to be a string")),
                    };
                    Ok(parse_whatdo(id, v)?)
                })
                .collect();

            Ok(Whatdo {
                id: String::from(id),
                summary: summary.clone(),
                whatdos: whatdos?,
            })
        }
        _ => Err(Error::msg("Whatdo data must be string or mapping")),
    }
}

fn parse_file(path: &Path) -> Result<Whatdo> {
    let file = std::fs::File::open(path)?;
    // let parsed: ParsedWhatdo = serde_yaml::from_slice(&file)?;
    let content: serde_yaml::Value = serde_yaml::from_reader(file)?;
    // let yaml_content = parser.load(file)?;
    let project_name = get_project_name(&path)?;

    return parse_whatdo(&project_name, &content);
}

fn read_current_file() -> Result<Whatdo> {
    let root: PathBuf = get_root()?;
    let path = root.join("WHATDO.yaml");
    return parse_file(&path);
}

pub fn add(id: &str) {
    println!("add {}", id)
}

pub fn list() {
    println!("list")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_file() {
        let parsed = parse_file(&PathBuf::from_str("./test_data/WHATDO.yaml").unwrap());
        assert_eq!(
            parsed.unwrap(),
            Whatdo {
                id: String::from("test_data"),
                summary: String::from(
                    "A streamlined git-based tool for task tracking of a project"
                ),
                whatdos: vec![]
            }
        );
    }
}
