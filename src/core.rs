use super::{git, sample};
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
    pub branch_name: Option<String>,
    pub simple_format: bool,
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
            branch_name: None,
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

    pub fn simple_format(&self) -> bool {
        self.simple_format
            && self.queue.is_none()
            && self.whatdos.is_none()
            && self.priority.is_none()
            && self.tags.is_none()
    }

    pub fn branch_name(&self) -> &String {
        self.branch_name.as_ref().unwrap_or(&self.id)
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
                        format!(
                            "{:>>width$}[{}]",
                            "",
                            id,
                            width = level - (unprinted_path.len() - i) - 1
                        )
                        .dimmed()
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

        for wd in whatdo.whatdos() {
            self.fmt_rec(
                f,
                &wd,
                unprinted_path,
                level + 1,
                transitively_satisfies_filter,
            )?;
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

fn validate_tag(tag: &str) -> Result<String> {
    if !TAG_RE.is_match(tag) {
        return Err(Error::msg("Tag must be of the form [a-z0-9-_]+"));
    }

    Ok(tag.to_owned())
}

fn validate_id(id: &str) -> Result<String> {
    if !ID_RE.is_match(id) {
        return Err(Error::msg("ID must be of the form [a-zA-Z0-9-_/]+"));
    }

    Ok(id.to_owned())
}

fn valid_tag(tag: &str) -> bool {
    validate_tag(tag).is_ok()
}

fn valid_id(id: &str) -> bool {
    validate_id(id).is_ok()
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
            let branch_name = match items.get("branch_name") {
                None => None,
                Some(p) => match p {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    _ => return Err(Error::msg("Expected 'branch_name' to be a string")),
                },
            };

            Ok(Whatdo {
                id: String::from(id),
                summary: summary.cloned(),
                whatdos: whatdos_map.map(parse_whatdo_map).transpose()?,
                queue: queue_sequence.map(parse_queue_sequence).transpose()?,
                tags: tags_sequence.map(parse_tags_sequence).transpose()?,
                priority,
                branch_name,
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

pub fn get_current_file() -> Result<PathBuf> {
    let root: PathBuf = git::get_root()?;
    Ok(root.join("WHATDO.yaml"))
}

fn read_current_file() -> Result<Whatdo> {
    return parse_file(&get_current_file()?);
}

fn serialize_whatdo(whatdo: &Whatdo) -> (serde_yaml::Value, serde_yaml::Value) {
    if whatdo.simple_format() {
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

    if let Some(priority) = whatdo.priority {
        mapping.insert(
            serde_yaml::Value::String(String::from("priority")),
            serde_yaml::Value::Number(Number::from(priority)),
        );
    }

    if let Some(branch_name) = &whatdo.branch_name {
        mapping.insert(
            serde_yaml::Value::String(String::from("branch_name")),
            serde_yaml::Value::String(branch_name.clone()),
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

fn find_whatdo_and_parent<'a, P: Fn(&Whatdo) -> bool>(
    root: &'a Whatdo,
    pred: &P,
) -> Option<(&'a Whatdo, Option<&'a Whatdo>)> {
    if pred(root) {
        return Some((root, None));
    }

    let whatdos = match &root.whatdos {
        None => return None,
        Some(wds) => wds,
    };

    for wd in whatdos {
        if let Some((wd, maybe_parent)) = find_whatdo_and_parent(&wd, pred) {
            return Some((wd, maybe_parent.or(Some(root))));
        }
    }

    return None;
}

fn find_whatdo_mut<'a, P: Fn(&Whatdo) -> bool>(
    root: &'a mut Whatdo,
    pred: &P,
) -> Option<&'a mut Whatdo> {
    if pred(root) {
        return Some(root);
    }

    let whatdos = match &mut root.whatdos {
        None => return None,
        Some(wds) => wds,
    };

    for wd in whatdos {
        if let Some(wd) = find_whatdo_mut(wd, pred) {
            return Some(wd);
        }
    }

    return None;
}

fn find_whatdo(root: &Whatdo, id: &str) -> Option<Whatdo> {
    return find_whatdo_and_parent(root, &|wd| wd.id == id)
        .map(|(wd, _)| wd)
        .cloned();
}

fn find_parent(root: &Whatdo, id: &str) -> Option<Whatdo> {
    return find_whatdo_and_parent(root, &|wd| wd.id == id)
        .and_then(|(_, parent)| parent)
        .cloned();
}

/// Return the first ancestor of the whatdo with the given id that
/// has a git branch
fn find_ancestor_with_branch(root: &Whatdo, id: &str) -> Result<Option<Whatdo>> {
    let mut current_id = id;

    loop {
        match find_whatdo_and_parent(root, &|wd| wd.id == current_id) {
            Some((_, Some(parent))) => {
                if git::branch_exists(&parent.branch_name())? {
                    return Ok(Some(parent.clone()));
                } else {
                    current_id = &parent.id;
                }
            }
            _ => {
                // We're at the root
                return Ok(None);
            }
        }
    }
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
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
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
        if satisfies_filter && !visited.contains(&wd.id) {
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
    branch_name: Option<String>,
    parent_id: Option<String>,
    commit: bool,
) -> Result<(Whatdo, Option<Whatdo>)> {
    let current_file = get_current_file()?;
    let mut whatdo = parse_file(&current_file)?;

    match find_whatdo_and_parent(&whatdo, &|wd| wd.id == id) {
        Some(_) => {
            return Err(Error::msg(format!(
                "Whatdo with ID '{}' already exists",
                id
            )))
        }
        None => {}
    }

    let validated_tags = tags
        .iter()
        .map(|t| validate_tag(&t))
        .collect::<Result<Vec<String>>>()?;

    let new_whatdo = Whatdo {
        id: validate_id(id)?,
        summary: summary.map(|s| s.to_owned()),
        simple_format: false,
        queue: None,
        whatdos: None,
        tags: if tags.len() > 0 {
            Some(validated_tags)
        } else {
            None
        },
        priority,
        branch_name,
    };

    match find_whatdo_and_parent(&whatdo, &|wd| new_whatdo.branch_name() == wd.branch_name()) {
        Some(_) => {
            return Err(Error::msg(format!(
                "Whatdo with branch name '{}' already exists",
                new_whatdo.branch_name()
            )))
        }
        None => {}
    }

    if git::branch_exists(new_whatdo.branch_name())? {
        return Err(Error::msg(format!("Branch with name '{}' already exists", new_whatdo.branch_name())));
    }

    let parent = {
        let parent_wd = if let Some(parent_id) = &parent_id {
            let normalized_parent_id = match parent_id.as_str() {
                "@" => match current()? {
                    None => return Err(Error::msg("No current whatdo to add to")),
                    Some(wd) => wd.id,
                },
                _ => parent_id.clone(),
            };
            match find_whatdo_mut(&mut whatdo, &|wd| &wd.id == &normalized_parent_id) {
                Some(wd) => wd,
                None => return Err(Error::msg("Parent not found")),
            }
        } else {
            &mut whatdo
        };
        if parent_wd.whatdos.is_none() {
            parent_wd.whatdos = Some(Vec::new());
        }
        parent_wd.whatdos.as_mut().unwrap().push(new_whatdo.clone());
        parent_id.map(|_| parent_wd).cloned()
    };
    write_to_file(&mut whatdo)?;

    if commit {
        git::commit([current_file], &format!("Add '{}' to whatdos", id), true)?;
    }

    Ok((new_whatdo, parent))
}

pub enum NextAmount {
    All,
    AtMost(usize),
}

pub fn next(amount: NextAmount, tags: Vec<String>, priorities: Vec<i64>) -> Result<Vec<Whatdo>> {
    let root = read_current_file()?;
    let current_wd = current()?;
    let mut visited = HashSet::new();
    if let Some(current_id) = current_wd.clone().map(|c| c.id) {
        visited.insert(current_id);
    }

    let filter = |wd: &Whatdo| {
        (tags.len() == 0
            || wd
                .tags
                .as_ref()
                .map(|ts| tags.iter().find(|t| ts.contains(t)))
                .is_some())
            && (priorities.len() == 0
                || (wd.priority.is_some() && priorities.contains(&wd.priority.unwrap())))
    };

    let mut current_sorted = if let Some(wd) = current_wd.clone() {
        sort_whatdos(&wd, &filter, &mut visited, false)
    } else {
        vec![]
    };

    let mut rest_sorted = sort_whatdos(&root, &filter, &mut visited, false);
    current_sorted.append(&mut rest_sorted);
    match amount {
        NextAmount::All => Ok(current_sorted),
        NextAmount::AtMost(n) => Ok(current_sorted.into_iter().take(n as usize).collect()),
    }
}

pub fn start(wd: &Whatdo) -> Result<()> {
    git::checkout_new_branch(wd.branch_name(), true)
}

pub fn get(id: &str) -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    Ok(find_whatdo(&whatdo, id))
}

pub fn root() -> Result<Option<Whatdo>> {
    let current_file = get_current_file()?;
    if !current_file.exists() {
        return Ok(None);
    }

    Ok(Some(read_current_file()?))
}

pub fn current() -> Result<Option<Whatdo>> {
    let whatdo = read_current_file()?;
    let current_branch = git::current_branch()?;
    if let Some((wd, _)) =
        find_whatdo_and_parent(&whatdo, &|wd| wd.branch_name() == &current_branch)
    {
        return Ok(Some(wd.clone()));
    }
    Ok(None)
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

pub fn delete(id: &str, commit: bool) -> Result<()> {
    let current_file = get_current_file()?;
    let whatdo = parse_file(&current_file)?;
    let new_whatdo = delete_whatdo(&whatdo, id);
    write_to_file(&new_whatdo)?;
    if commit {
        git::commit(
            [current_file],
            &format!("Deleted '{}' from whatdos", id),
            true,
        )?;
    }
    Ok(())
}

pub fn resolve(id: &str, commit: bool) -> Result<()> {
    let current_file = get_current_file()?;
    let whatdo = parse_file(&current_file)?;
    let new_whatdo = delete_whatdo(&whatdo, id);
    write_to_file(&new_whatdo)?;
    if commit {
        git::commit([current_file], &format!("Resolved whatdo '{}'", id), true)?;
    }
    Ok(())
}

pub fn finish(commit: bool, merge: bool) -> Result<()> {
    let current_file = get_current_file()?;
    let whatdo = parse_file(&current_file)?;
    let current_wd = match current()? {
        None => return Err(Error::msg("No active whatdo")),
        Some(wd) => wd,
    };
    let target_branch = find_ancestor_with_branch(&whatdo, &current_wd.id)?
        .and_then(|p| {
            if p.id == whatdo.id {
                whatdo.branch_name.clone()
            } else {
                Some(p.branch_name().to_owned())
            }
        })
        .unwrap_or(git::default_branch_name()?);
    if merge && git::has_unstaged_changes()? {
        return Err(Error::msg(
            "You have unstaged changes. Commit or revert them before finishing whatdo",
        ));
    }
    let new_whatdo = delete_whatdo(&whatdo, &current_wd.id);
    write_to_file(&new_whatdo)?;
    if commit {
        git::commit(
            [current_file],
            &format!("Finished whatdo '{}'", &current_wd.id),
            true,
        )?;
    }
    if merge {
        git::merge(&target_branch, true)?;
    }
    Ok(())
}

pub fn init() -> Result<PathBuf> {
    let current_file = get_current_file()?;
    if current_file.exists() {
        return Err(Error::msg(format!(
            "Whatdo file already exists at {}",
            current_file.to_string_lossy()
        )));
    }

    let initial_content = sample::initial_whatdo_file();
    write_to_file(&initial_content)?;
    Ok(current_file)
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
                        branch_name: None,
                        tags: Some(vec!["a-tag".to_owned()]),
                    },
                ]),
                queue: None,
                priority: Some(0),
                tags: None,
                branch_name: None,
                simple_format: false,
            }]),
            simple_format: false,
            queue: Some(vec![
                String::from("read-back-whatdos"),
                String::from("delete-whatdo"),
            ]),
            priority: None,
            tags: None,
            branch_name: Some(String::from("overridden-name")),
        }
    }

    #[test]
    fn test_parse_file() {
        let parsed = parse_file(&PathBuf::from("./test_data/WHATDO.yaml"));
        assert_eq!(parsed.unwrap(), test_data_whatdo());
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
                            branch_name: None,
                            tags: Some(vec!["a-tag".to_owned()]),
                        },
                    ]),
                    queue: None,
                    priority: Some(0),
                    tags: None,
                    branch_name: None,
                    simple_format: false,
                }]),
                simple_format: false,
                queue: Some(vec![String::from("read-back-whatdos")]),
                priority: None,
                tags: None,
                branch_name: Some(String::from("overridden-name")),
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
                        branch_name: None,
                        tags: Some(vec!["a-tag".to_owned()]),
                    },]),
                    queue: None,
                    priority: Some(0),
                    tags: None,
                    branch_name: None,
                    simple_format: false,
                }]),
                simple_format: false,
                queue: Some(vec![]),
                priority: None,
                tags: None,
                branch_name: Some(String::from("overridden-name")),
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
        let sorted = sort_whatdos(&whatdo, &|_| true, &mut HashSet::new(), false);
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
