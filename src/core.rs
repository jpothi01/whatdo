use super::git;
use anyhow::{Error, Result};
use core::fmt;
use serde_yaml::Mapping;
use std::path::PathBuf;
use std::path::{Component, Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Whatdo {
    id: String,
    summary: Option<String>,
    whatdos: Option<Vec<Whatdo>>,
    queue: Option<Vec<String>>,
    simple_format: bool,
}

fn deslugify(s: &str) -> String {
    let mut result = String::new();
    let mut first = true;
    for char in s.chars() {
        match char {
            '_' | '-' => result.push(' '),
            _ => {
                if first {
                    result.push_str(&char.to_uppercase().to_string())
                } else {
                    result.push(char)
                }
            }
        }
        first = false
    }

    result
}

impl Whatdo {
    pub fn simple<T: Into<String>, U: Into<String>>(id: T, summary: U) -> Self {
        Whatdo {
            id: id.into(),
            summary: Some(summary.into()),
            whatdos: None,
            queue: None,
            simple_format: true,
        }
    }

    fn summary(&self) -> String {
        match &self.summary {
            Some(s) => s.clone(),
            None => deslugify(&self.id),
        }
    }

    fn whatdos(&self) -> Vec<Whatdo> {
        match &self.whatdos {
            None => Vec::new(),
            Some(wds) => wds.clone(),
        }
    }
}

impl fmt::Display for Whatdo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.id, self.summary())
    }
}

fn get_project_name(path: &Path) -> Result<String> {
    match path.components().nth_back(1) {
        Some(Component::Normal(n)) => Ok(n.to_str().unwrap().to_owned()),
        _ => Err(Error::msg("Cannot use wd from the root of the filesystem")),
    }
}

fn parse_whatdo_map(mapping: serde_yaml::Mapping) -> Result<Vec<Whatdo>> {
    mapping
        .iter()
        .map(|(k, v)| {
            let id = match k {
                serde_yaml::Value::String(s) => s,
                _ => return Err(Error::msg("Expected mapping key to be a string")),
            };
            Ok(parse_whatdo(id, v)?)
        })
        .collect()
}

fn parse_queue_sequence(list: serde_yaml::Sequence) -> Result<Vec<String>> {
    list.iter()
        .map(|v| {
            let id = match v {
                serde_yaml::Value::String(s) => s,
                _ => return Err(Error::msg("Expected sequence item to be a string")),
            };
            Ok(id.clone())
        })
        .collect()
}

fn parse_whatdo(id: &str, data: &serde_yaml::Value) -> Result<Whatdo> {
    match data {
        serde_yaml::Value::String(s) => Ok(Whatdo::simple(id.to_owned(), s.clone())),
        serde_yaml::Value::Mapping(items) => {
            let summary = match items.get("summary") {
                None => None,
                Some(s) => match s {
                    serde_yaml::Value::String(s) => Some(s),
                    _ => return Err(Error::msg("Expected 'summary' to be a string")),
                },
            };
            let whatdos_map = match items.get("whatdos") {
                None => None,
                Some(d) => match d {
                    serde_yaml::Value::Mapping(d) => Some(d.clone()),
                    _ => return Err(Error::msg("Expected 'whatdos' to be a mapping")),
                },
            };
            let queue_sequence = match items.get("queue") {
                None => None,
                Some(d) => match d {
                    serde_yaml::Value::Sequence(s) => Some(s.clone()),
                    _ => return Err(Error::msg("Expected 'queue' to be a sequence")),
                },
            };

            Ok(Whatdo {
                id: String::from(id),
                summary: summary.cloned(),
                whatdos: whatdos_map.map(parse_whatdo_map).transpose()?,
                queue: queue_sequence.map(parse_queue_sequence).transpose()?,
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
    let root: PathBuf = git::get_root()?;
    Ok(root.join("WHATDO.yaml"))
}

fn read_current_file() -> Result<Whatdo> {
    return parse_file(&get_current_file()?);
}

fn serialize_whatdo(whatdo: &Whatdo) -> (serde_yaml::Value, serde_yaml::Value) {
    if whatdo.simple_format {
        let summary_value = if let Some(summary) = whatdo.summary.clone() {
            serde_yaml::Value::String(summary)
        } else {
            serde_yaml::Value::Mapping(Mapping::new())
        };
        return (serde_yaml::Value::String(whatdo.id.clone()), summary_value);
    }

    let mut mapping = serde_yaml::Mapping::new();
    if let Some(summary) = whatdo.summary.clone() {
        mapping.insert(
            serde_yaml::Value::String(String::from("summary")),
            serde_yaml::Value::String(summary),
        );
    }

    if let Some(whatdos) = whatdo.whatdos.clone() {
        if whatdos.len() > 0 {
            let mut whatdo_mapping = serde_yaml::Mapping::new();
            for subwhatdo in &whatdos {
                let (k, v) = serialize_whatdo(&subwhatdo);
                whatdo_mapping.insert(k, v);
            }

            mapping.insert(
                serde_yaml::Value::String(String::from("whatdos")),
                serde_yaml::Value::Mapping(whatdo_mapping),
            );
        }
    }

    if let Some(queue) = whatdo.queue.clone() {
        if queue.len() > 0 {
            mapping.insert(
                serde_yaml::Value::String(String::from("queue")),
                serde_yaml::Value::Sequence(
                    queue
                        .into_iter()
                        .map(|i| serde_yaml::Value::String(i))
                        .collect(),
                ),
            );
        }
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
    if whatdo.whatdos.is_none() {
        whatdo.whatdos = Some(Vec::new());
    }
    whatdo.whatdos.as_mut().unwrap().push(new_whatdo);
    write_to_file(&whatdo)?;

    Ok(())
}

pub fn list() {
    println!("list")
}

fn find_whatdo(root: &Whatdo, id: &str) -> Option<Whatdo> {
    if root.id == id {
        return Some(root.clone());
    }

    let whatdos = match &root.whatdos {
        None => return None,
        Some(wds) => wds,
    };

    for wd in whatdos {
        if let Some(found) = find_whatdo(&wd, id) {
            return Some(found);
        }
    }

    return None;
}

fn next_whatdo(wd: &Whatdo) -> Option<Whatdo> {
    if let Some(queue) = &wd.queue {
        if queue.len() > 0 {
            return find_whatdo(wd, &queue[0]);
        }
    }

    let whatdos = wd.whatdos();
    if whatdos.len() == 0 {
        return Some(wd.clone());
    }

    return next_whatdo(&whatdos[0]);
}

pub fn next() -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    Ok(next_whatdo(&whatdo))
}

pub fn start(wd: &Whatdo) -> Result<()> {
    git::checkout_new_branch(&wd.id)
}

pub fn get(id: &str) -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    Ok(find_whatdo(&whatdo, id))
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn test_data_whatdo() -> Whatdo {
        Whatdo {
            id: String::from("test_data"),
            summary: Some(String::from(
                "A streamlined git-based tool for task tracking of a project",
            )),
            whatdos: Some(vec![Whatdo {
                id: String::from("basic-functionality"),
                summary: Some(String::from(
                    "Implement the absolute minimum stuff for the tool to get it to be useful
for tracking the progress of this tool\n",
                )),
                whatdos: Some(vec![
                    Whatdo::simple(
                        String::from("read-back-whatdos"),
                        String::from("Ability to invoke `wd` to list the current whatdos"),
                    ),
                    Whatdo {
                        id: String::from("finish-whatdo"),
                        summary: Some(String::from(
                            "Ability to invoke `wd finish` to finish the current whatdo",
                        )),
                        whatdos: Some(vec![Whatdo::simple("delete-whatdo", "Delete the whatdo")]),
                        simple_format: false,
                        queue: None,
                    },
                ]),
                queue: None,
                simple_format: false,
            }]),
            simple_format: false,
            queue: Some(vec![
                String::from("read-back-whatdos"),
                String::from("delete-whatdo"),
            ]),
        }
    }

    #[test]
    fn test_parse_file() {
        let parsed = parse_file(&PathBuf::from("./test_data/WHATDO.yaml"));
        assert_eq!(parsed.unwrap(), test_data_whatdo());
    }

    #[test]
    fn test_next_whatdo() {
        assert_eq!(
            next_whatdo(&test_data_whatdo()),
            Some(Whatdo::simple(
                String::from("read-back-whatdos"),
                String::from("Ability to invoke `wd` to list the current whatdos"),
            ))
        )
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
