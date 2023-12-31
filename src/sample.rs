use crate::core::Whatdo;

pub fn initial_whatdo_file() -> Whatdo {
    Whatdo {
        id: String::from("root"),
        simple_format: false,
        summary: Some(String::from("<description of your project>")),
        queue: Some(vec![String::from("setting-up-new-project")]),
        priority: None,
        tags: None,
        whatdos: Some(vec![
            Whatdo {
                id: String::from("setting-up-new-project"),
                priority: Some(1),
                queue: None,
                summary: Some(String::from(
                    "Things to do to set up your WHATDO.yaml for a project",
                )),
                tags: None,
                simple_format: false,
                whatdos: Some(vec![
                    Whatdo {
                        id: String::from("run-start-command"),
                        priority: None,
                        queue: None,
                        summary: Some(String::from(
                            "Start this interactive tutorial with `wd start setting-up-new-project`",
                        )),
                        tags: None,
                        whatdos: None,
                        simple_format: false,
                    },
                    Whatdo {
                        id: String::from("use-next-command"),
                        priority: None,
                        queue: None,
                        summary: Some(String::from(
                            "View what to do next with `wd next`, or view the whole whatdo tree with `wd ls`",
                        )),
                        tags: None,
                        whatdos: None,
                        simple_format: false,
                    },
                    Whatdo {
                    id: String::from("add-with-cli"),
                    priority: None,
                    queue: None,
                    summary: Some(String::from(
                        "Add some real whatdos: `wd add example-whatdo-id -m \"Long form description of what to do\"`",
                    )),
                    tags: None,
                    whatdos: None,
                    simple_format: false,
                },
                Whatdo {
                    id: String::from("add-manually"),
                    priority: None,
                    queue: None,
                    summary: Some(String::from(
                        "Add abbreviated whatdos like this by manually editing this file",
                    )),
                    tags: None,
                    whatdos: None,
                    simple_format: true,
                },
                Whatdo {
                    id: String::from("use-tags"),
                    priority: Some(2),
                    queue: None,
                    summary: Some(String::from(
                        "Classify whatdos with tags and priorities: `wd add test-tags --tags important,cool -p 1",
                    )),
                    tags: Some(vec![String::from("optional")]),
                    whatdos: None,
                    simple_format: false,
                },
                Whatdo {
                    id: String::from("nest"),
                    priority: None,
                    queue: None,
                    summary: Some(String::from(
                        "Nest whatdos: `wd add sub-whatdo --parent example-whatdo-id`",
                    )),
                    tags: Some(vec![String::from("optional")]),
                    whatdos: None,
                    simple_format: false,
                },
                Whatdo {
                    id: String::from("run-finish-command"),
                    priority: None,
                    queue: None,
                    summary: Some(String::from(
                        "Finish this tutorial and merge changes to the default branch: `wd finish`",
                    )),
                    tags: None,
                    whatdos: None,
                    simple_format: false,
                }]),
            }
        ])
    }
}
