use super::git;
use anyhow::{Error, Result};
use colored::Colorize;
use core::fmt;
use log::warn;
use once_cell::sync::Lazy;
use serde_yaml::{Mapping, Number};
use std::collections::HashSet;
use std::path::PathBuf;
use std::path::{Component, Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Whatdo {
    pub id: String,
    pub summary: Option<String>,
    pub whatdos: Option<Vec<Whatdo>>,
    pub queue: Option<Vec<String>>,
    pub priority: Option<i64>,
    pub tags: Option<Vec<String>>,
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
    pub fn simple<T: Into<String>, U: Into<String>>(id: T, summary: Option<U>) -> Self {
        Whatdo {
            id: id.into(),
            summary: summary.map(|s| s.into()),
            whatdos: None,
            queue: None,
            priority: None,
            tags: None,
            simple_format: true,
        }
    }

    pub fn summary(&self) -> String {
        match &self.summary {
            Some(s) => s.clone(),
            None => deslugify(&self.id),
        }
    }

    pub fn whatdos(&self) -> Vec<Whatdo> {
        match &self.whatdos {
            None => Vec::new(),
            Some(wds) => wds.clone(),
        }
    }
}

impl fmt::Display for Whatdo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.id.yellow())?;
        if let Some(p) = self.priority {
            write!(f, " [P{}]", p.to_string().bold())?;
        }
        if let Some(tags) = &self.tags {
            write!(f, " [")?;
            let mut first = true;
            for tag in tags {
                write!(f, "{}", tag)?;
                if !first {
                    write!(f, ",")?;
                }
                first = false;
            }
            write!(f, "]")?;
        }
        write!(f, " {}", self.summary())
    }
}

pub struct WhatdoDetail(pub Whatdo);

struct WhatdoNode {
    whatdo: Whatdo,
    level: usize,
    children: Vec<Whatdo>,
}

pub struct WhatdoTreeView {
    pub root: Whatdo,
    pub filter: Box<dyn Fn(&Whatdo) -> bool>,
    // If true, all children of selected nodes will be printed
    pub transitive: bool,
}

impl WhatdoTreeView {
    fn fmt_rec(
        &self,
        f: &mut fmt::Formatter<'_>,
        whatdo: &Whatdo,
        unprinted_path: &mut Vec<String>,
        level: usize,
        ancestor_satisfied_filter: bool,
    ) -> fmt::Result {
        let satisfies_filter = (*self.filter)(whatdo);
        let transitively_satisfies_filter =
            satisfies_filter || self.transitive && ancestor_satisfied_filter;

        if whatdo.id != self.root.id {
            if transitively_satisfies_filter {
                for (i, id) in unprinted_path.iter().enumerate() {
                    writeln!(
                        f,
                        "{}",
                        format!("{:>>width$}[{}]", "", id, width = level - i - 2).dimmed()
                    )?;
                }
                unprinted_path.clear();
            }

            if satisfies_filter {
                writeln!(
                    f,
                    "{}",
                    format!("{:>>width$}{}", "", whatdo, width = level - 1)
                )?;
            } else if transitively_satisfies_filter {
                writeln!(
                    f,
                    "{}",
                    format!("{:>>width$}[{}]", "", whatdo.id, width = level - 1).dimmed()
                )?;
            } else {
                unprinted_path.push(whatdo.id.clone());
            }
        }

        if let Some(whatdos) = whatdo.whatdos.as_ref().filter(|wds| wds.len() > 0) {
            for wd in whatdos {
                self.fmt_rec(
                    f,
                    wd,
                    unprinted_path,
                    level + 1,
                    transitively_satisfies_filter,
                )?;
            }
        }

        // If none of our children cleared the unprinted path,
        // remove ourself from the unprinted path
        if unprinted_path.last() == Some(&whatdo.id) {
            unprinted_path.remove(unprinted_path.len() - 1);
        }

        Ok(())
    }
}

impl<'a> fmt::Display for WhatdoTreeView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_rec(f, &self.root, &mut vec![], 0, false)
    }
}

static TAG_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new("^[a-z0-9-_]+$").unwrap());
static ID_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new("^[a-zA-Z0-9-_/]+$").unwrap());

fn valid_tag(tag: &str) -> bool {
    return TAG_RE.is_match(tag);
}

fn valid_id(id: &str) -> bool {
    return ID_RE.is_match(id);
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
                serde_yaml::Value::String(s) => {
                    if valid_id(&s) {
                        s
                    } else {
                        return Err(Error::msg(format!("Invalid whatdo ID: {}", s)));
                    }
                }
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
                serde_yaml::Value::String(s) => {
                    if valid_id(&s) {
                        s
                    } else {
                        return Err(Error::msg(format!("Invalid whatdo ID: {}", s)));
                    }
                }
                _ => return Err(Error::msg("Expected sequence item to be a string")),
            };
            Ok(id.clone())
        })
        .collect()
}

fn parse_tags_sequence(list: serde_yaml::Sequence) -> Result<Vec<String>> {
    list.iter()
        .map(|v| {
            let id = match v {
                serde_yaml::Value::String(s) => {
                    if valid_tag(&s) {
                        s
                    } else {
                        return Err(Error::msg(format!("Invalid tag: {}", s)));
                    }
                }
                _ => return Err(Error::msg("Expected sequence item to be a string")),
            };
            Ok(id.clone())
        })
        .collect()
}

fn parse_whatdo(id: &str, data: &serde_yaml::Value) -> Result<Whatdo> {
    match data {
        serde_yaml::Value::String(s) => Ok(Whatdo::simple(id.to_owned(), Some(s.clone()))),
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
            let priority = match items.get("priority") {
                None => None,
                Some(p) => match p {
                    serde_yaml::Value::Number(n) => match n.as_i64() {
                        None => return Err(Error::msg("Expected 'priority' to be an integer")),
                        Some(n) => Some(n),
                    },
                    _ => return Err(Error::msg("Expected 'priority' to be a number")),
                },
            };
            let tags_sequence = match items.get("tags") {
                None => None,
                Some(d) => match d {
                    serde_yaml::Value::Sequence(s) => Some(s.clone()),
                    _ => return Err(Error::msg("Expected 'tags' to be a sequence")),
                },
            };

            Ok(Whatdo {
                id: String::from(id),
                summary: summary.cloned(),
                whatdos: whatdos_map.map(parse_whatdo_map).transpose()?,
                queue: queue_sequence.map(parse_queue_sequence).transpose()?,
                tags: tags_sequence.map(parse_tags_sequence).transpose()?,
                priority,
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

    if let Some(queue) = whatdo.queue.clone() {
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

    if let Some(priority) = whatdo.priority {
        mapping.insert(
            serde_yaml::Value::String(String::from("priority")),
            serde_yaml::Value::Number(Number::from(priority)),
        );
    }

    if let Some(tags) = whatdo.tags.clone() {
        mapping.insert(
            serde_yaml::Value::String(String::from("tags")),
            serde_yaml::Value::Sequence(
                tags.into_iter()
                    .map(|i| serde_yaml::Value::String(i))
                    .collect(),
            ),
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

/// Return all whatdos descedent from the given whatdo in the order
/// defined by the prioritization algorithm.
/// Ignore any whatdos (or whatdo trees) for which filter(wd) returns false
/// Ignore any whatdos that have already been added to visited
fn sort_whatdos<F: Fn(&Whatdo) -> bool>(
    wd: &Whatdo,
    filter: &F,
    visited: &mut HashSet<String>,
    ancestor_satisfies_filter: bool,
) -> Vec<Whatdo> {
    let mut result: Vec<Whatdo> = Vec::new();

    let satisfies_filter = filter(wd) || ancestor_satisfies_filter;

    if let Some(queue) = &wd.queue {
        for id in queue {
            if visited.contains(id) {
                continue;
            }

            let queue_wd = match find_whatdo(wd, id) {
                Some(wd) => wd,
                None => {
                    warn!("Queue item not found in {}: {}", wd.id, id);
                    continue;
                }
            };

            let mut other = sort_whatdos(&queue_wd, filter, visited, satisfies_filter);
            result.append(&mut other);
            visited.insert(id.clone());
        }
    }

    if let Some(mut whatdos) = wd.whatdos.clone().filter(|wd| wd.len() > 0) {
        whatdos.sort_by(|a, b| match (a.priority, b.priority) {
            (Some(pa), Some(pb)) => pa.cmp(&pb),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        for subwhatdo in whatdos {
            if visited.contains(&subwhatdo.id) {
                continue;
            }

            let mut other = sort_whatdos(&subwhatdo, filter, visited, satisfies_filter);
            result.append(&mut other);
            visited.insert(subwhatdo.id.clone());
        }
    } else {
        // Base case
        if satisfies_filter {
            result.push(wd.clone());
        }
    }

    return result;
}

pub fn add(
    id: &str,
    tags: Vec<String>,
    summary: Option<&str>,
    priority: Option<i64>,
    parent: Option<String>,
) -> Result<()> {
    let mut whatdo = read_current_file()?;
    let new_whatdo = Whatdo {
        id: id.to_owned(),
        summary: summary.map(|s| s.to_owned()),
        simple_format: false,
        queue: None,
        whatdos: None,
        tags: if tags.len() > 0 { Some(tags) } else { None },
        priority,
    };
    if whatdo.whatdos.is_none() {
        whatdo.whatdos = Some(Vec::new());
    }
    whatdo.whatdos.as_mut().unwrap().push(new_whatdo);
    write_to_file(&whatdo)?;

    Ok(())
}

pub enum NextAmount {
    All,
    AtMost(usize),
}

pub fn next(amount: NextAmount, tags: Vec<String>) -> Result<Vec<Whatdo>> {
    let root = read_current_file()?;
    let next_root = if let Some(wd) = current()? {
        wd
    } else {
        root
    };
    let sorted = sort_whatdos(
        &next_root,
        &|wd| {
            tags.len() == 0
                || wd
                    .tags
                    .as_ref()
                    .map(|ts| tags.iter().find(|t| ts.contains(t)))
                    .is_some()
        },
        &mut HashSet::new(),
        false,
    );
    match amount {
        NextAmount::All => Ok(sorted),
        NextAmount::AtMost(n) => Ok(sorted.into_iter().take(n as usize).collect()),
    }
}

pub fn start(wd: &Whatdo) -> Result<()> {
    git::checkout_new_branch(&wd.id)
}

pub fn get(id: &str) -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    Ok(find_whatdo(&whatdo, id))
}

pub fn root() -> Result<Whatdo> {
    read_current_file()
}

pub fn current() -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    let current_id = git::current_branch()?;
    Ok(find_whatdo(&whatdo, &current_id))
}

fn delete_whatdo(whatdo: &Whatdo, id: &str) -> Whatdo {
    debug_assert!(whatdo.id != id);
    let mut new_whatdo = whatdo.clone();
    if let Some(queue) = &mut new_whatdo.queue {
        let found = queue.iter().position(|i| i == id);
        if let Some(found) = found {
            queue.remove(found);
        }
    }

    if let Some(whatdos) = &mut new_whatdo.whatdos {
        let found = whatdos.iter().position(|wd| wd.id == id);
        if let Some(found) = found {
            whatdos.remove(found);
        }
    }

    new_whatdo.whatdos = new_whatdo
        .whatdos
        .map(|whatdos| whatdos.iter().map(|wd| delete_whatdo(wd, id)).collect());

    return new_whatdo;
}

pub fn delete(id: &str) -> Result<()> {
    let whatdo = read_current_file()?;
    let new_whatdo = delete_whatdo(&whatdo, id);
    write_to_file(&new_whatdo)?;
    Ok(())
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
                        Some(String::from(
                            "Ability to invoke `wd` to list the current whatdos",
                        )),
                    ),
                    Whatdo {
                        id: String::from("finish-whatdo"),
                        summary: Some(String::from(
                            "Ability to invoke `wd finish` to finish the current whatdo",
                        )),
                        whatdos: Some(vec![Whatdo::simple(
                            "delete-whatdo",
                            Some("Delete the whatdo"),
                        )]),
                        simple_format: false,
                        queue: None,
                        priority: None,
                        tags: Some(vec!["a-tag".to_owned()]),
                    },
                ]),
                queue: None,
                priority: Some(0),
                tags: None,
                simple_format: false,
            }]),
            simple_format: false,
            queue: Some(vec![
                String::from("read-back-whatdos"),
                String::from("delete-whatdo"),
            ]),
            priority: None,
            tags: None,
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
                Some(String::from(
                    "Ability to invoke `wd` to list the current whatdos"
                )),
            ))
        )
    }

    #[test]
    fn test_delete_whatdo() {
        let deleted = delete_whatdo(&test_data_whatdo(), "delete-whatdo");
        assert_eq!(
            deleted,
            Whatdo {
                id: String::from("test_data"),
                summary: Some(String::from(
                    "A streamlined git-based tool for task tracking of a project",
                )),
                whatdos: Some(vec![Whatdo {
                    id: String::from("basic-functionality"),
                    summary: Some(String::from(
                        "Implement the absolute minimum stuff for the tool to get it to be useful\nfor tracking the progress of this tool\n",
                    )),
                    whatdos: Some(vec![
                        Whatdo::simple(
                            String::from("read-back-whatdos"),
                            Some(String::from(
                                "Ability to invoke `wd` to list the current whatdos"
                            )),
                        ),
                        Whatdo {
                            id: String::from("finish-whatdo"),
                            summary: Some(String::from(
                                "Ability to invoke `wd finish` to finish the current whatdo",
                            )),
                            whatdos: Some(vec![]),
                            simple_format: false,
                            queue: None,
                            priority: None,
                            tags: Some(vec!["a-tag".to_owned()]),
                        },
                    ]),
                    queue: None,
                    priority: Some(0),
                    tags: None,
                    simple_format: false,
                }]),
                simple_format: false,
                queue: Some(vec![String::from("read-back-whatdos")]),
                priority: None,
                tags: None,
            }
        );
        let deleted_again = delete_whatdo(&deleted, "read-back-whatdos");
        assert_eq!(
            deleted_again,
            Whatdo {
                id: String::from("test_data"),
                summary: Some(String::from(
                    "A streamlined git-based tool for task tracking of a project",
                )),
                whatdos: Some(vec![Whatdo {
                    id: String::from("basic-functionality"),
                    summary: Some(String::from(
                        "Implement the absolute minimum stuff for the tool to get it to be useful\nfor tracking the progress of this tool\n",
                    )),
                    whatdos: Some(vec![Whatdo {
                        id: String::from("finish-whatdo"),
                        summary: Some(String::from(
                            "Ability to invoke `wd finish` to finish the current whatdo",
                        )),
                        whatdos: Some(vec![]),
                        simple_format: false,
                        queue: None,
                        priority: None,
                        tags: Some(vec!["a-tag".to_owned()]),
                    },]),
                    queue: None,
                    priority: Some(0),
                    tags: None,
                    simple_format: false,
                }]),
                simple_format: false,
                queue: Some(vec![]),
                priority: None,
                tags: None,
            }
        );
    }

    #[test]
    fn test_serialize() {
        let serialized = serialize_whatdo(&test_data_whatdo());
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string("./test_data/WHATDO.yaml").unwrap())
                .unwrap();
        assert_eq!(serialized.1, parsed);
    }

    #[test]
    fn test_sort_whatdos() {
        let whatdo = parse_file(Path::new("./test_data/sort_test.yaml")).unwrap();
        let sorted = sort_whatdos(&whatdo, &|wd| true, &mut HashSet::new(), false);
        assert_eq!(
            sorted.iter().map(|wd| &wd.id).collect::<Vec<_>>(),
            vec![
                "read-back-whatdos",
                "delete-whatdo",
                "read-users-mind",
                "less-fossil-fuels",
                "more-green-energy",
            ]
        );

        let sorted_tags = sort_whatdos(
            &whatdo,
            &|wd| {
                wd.tags
                    .as_ref()
                    .map(|tags| tags.iter().find(|t| t.as_str() == "todo"))
                    .is_some()
            },
            &mut HashSet::new(),
            false,
        );
        assert_eq!(
            sorted_tags.iter().map(|wd| &wd.id).collect::<Vec<_>>(),
            vec!["delete-whatdo", "more-green-energy",]
        )
    }
}
