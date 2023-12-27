use anyhow::{Error, Result};
use serde_derive::{Deserialize, Serialize};
use std::path::{Component, Path};
use std::process::Command;
use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};

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
    simple_format: bool,
}

impl Whatdo {
    pub fn simple<T: Into<String>, U: Into<String>>(id: T, summary: U) -> Self {
        Whatdo {
            id: id.into(),
            summary: summary.into(),
            whatdos: vec![],
            simple_format: true,
        }
    }
}

fn get_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    return Ok(PathBuf::from(
        &String::from_utf8(output.stdout).unwrap().trim(),
    ));
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
                simple_format: false,
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

fn get_current_file() -> Result<PathBuf> {
    let root: PathBuf = get_root()?;
    Ok(root.join("WHATDO.yaml"))
}

fn read_current_file() -> Result<Whatdo> {
    return parse_file(&get_current_file()?);
}

fn serialize_whatdo(whatdo: &Whatdo) -> (serde_yaml::Value, serde_yaml::Value) {
    if whatdo.simple_format {
        return (
            serde_yaml::Value::String(whatdo.id.clone()),
            serde_yaml::Value::String(whatdo.summary.clone()),
        );
    }

    let mut mapping = serde_yaml::Mapping::new();
    mapping.insert(
        serde_yaml::Value::String(String::from("summary")),
        serde_yaml::Value::String(whatdo.summary.clone()),
    );
    if whatdo.whatdos.len() > 0 {
        let mut whatdo_mapping = serde_yaml::Mapping::new();
        for subwhatdo in &whatdo.whatdos {
            let (k, v) = serialize_whatdo(&subwhatdo);
            whatdo_mapping.insert(k, v);
        }

        mapping.insert(
            serde_yaml::Value::String(String::from("whatdos")),
            serde_yaml::Value::Mapping(whatdo_mapping),
        );
    }

    return (
        serde_yaml::Value::String(whatdo.id.clone()),
        serde_yaml::Value::Mapping(mapping),
    );
}

fn write_to_file(whatdo: &Whatdo) -> Result<()> {
    let path = get_current_file()?;
    let serialized = serialize_whatdo(whatdo);
    let file = std::fs::File::create(path)?;
    serde_yaml::to_writer(file, &serialized.1)?;
    Ok(())
}

pub fn add(id: &str, summary: &str) -> Result<()> {
    let mut whatdo = read_current_file()?;
    let new_whatdo = Whatdo::simple(id, summary);
    whatdo.whatdos.push(new_whatdo);
    write_to_file(&whatdo)?;

    Ok(())
}

pub fn list() {
    println!("list")
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_data_whatdo() -> Whatdo {
        Whatdo {
            id: String::from("test_data"),
            summary: String::from("A streamlined git-based tool for task tracking of a project"),
            whatdos: vec![Whatdo {
                id: String::from("basic-functionality"),
                summary: String::from(
                    "Implement the absolute minimum stuff for the tool to get it to be useful
for tracking the progress of this tool\n",
                ),
                whatdos: vec![
                    Whatdo::simple(
                        String::from("read-back-whatdos"),
                        String::from("Ability to invoke `wd` to list the current whatdos"),
                    ),
                    Whatdo {
                        id: String::from("finish-whatdo"),
                        summary: String::from(
                            "Ability to invoke `wd finish` to finish the current whatdo",
                        ),
                        whatdos: vec![Whatdo::simple("delete-whatdo", "Delete the whatdo")],
                        simple_format: false,
                    },
                ],
                simple_format: false,
            }],
            simple_format: false,
        }
    }

    #[test]
    fn test_parse_file() {
        let parsed = parse_file(&PathBuf::from_str("./test_data/WHATDO.yaml").unwrap());
        assert_eq!(parsed.unwrap(), test_data_whatdo());
    }

    #[test]
    fn test_serialize() {
        let serialized = serialize_whatdo(&test_data_whatdo());
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string("./test_data/WHATDO.yaml").unwrap())
                .unwrap();
        assert_eq!(serialized.1, parsed);
    }
}
