# whatdo - Project management for personal projects

`whatdo` (or `wd`, as the executable is named) is a YAML file format and CLI tool for keeping track of tasks
in a git-based, single-person project.

**SUPER ALPHA**
This tool is one week old and is haphazardly put together

# Install

Currently only available to install from source via `cargo`

```
cargo install --git git@github.com:jpothi01/whatdo.git --branch release
```

# At a glance

The file format:

```YAML
summary: The tutorial for wd
# The 'queue' is a manually set sequence of tasks to complete
queue:
- setting-up-new-project
whatdos:
    # Every task (whatdo) is either a leaf whatdo or has sub-whatdo.
    # This whatdo, called "setting-up-new-project", has sub-whatdos.
    # `wd next` traverses this tree to determine what you should do next
    # based on the queue, the order of sub-whatdos, priority, and tag filters.
  setting-up-new-project:
    summary: Things to do to set up your WHATDO.yaml for a project
    # Use priority to influence the output of `wd next`.
    # Lower-numbered priority whatdos are ordered before high-numbered ones
    priority: 1
    whatdos:
      run-start-command:
        summary: Start this interactive tutorial with `wd start setting-up-new-project`
      use-next-command:
        summary: View what to do next with `wd next`, or view the whole whatdo tree with `wd ls`
      add-with-cli:
        summary: Add some real whatdos with `wd add example-whatdo-id`
      add-manually: You can abbreviated whatdos by manually editing this file
      use-tags:
        summary: Use tags to classify whatdos
        # `wd ls --tags optional` would output this whatdo and the one below it
        tags:
        - optional
      nest:
        summary: Nest whatdos with `wd add sub-whatdo --parent example-whatdo-id`
        tags:
        - optional
      run-finish-command:
        summary: 'Finish this tutorial and merge changes to the default branch: `wd finish`'
```

The CLI:

```
~/next-big-app (master)> wd init
Whatdo file initialized at:
/Users/john/code/next-big-app/WHATDO.yaml

Run `wd` to get started
~/next-big-app (master)> wd
No active whatdo

Next few whatdos:
[run-start-command] Start this interactive tutorial with `wd start setting-up-new-project`
[use-next-command] View what to do next with `wd next`, or view the whole whatdo tree with `wd ls`
[add-with-cli] Add some real whatdos with `wd add example-whatdo-id`
[add-manually] You can abbreviated whatdos by manually editing this file
[use-tags] [optional] Use tags to classify whatdos
[nest] [optional] Nest whatdos with `wd add sub-whatdo --parent example-whatdo-id`
[run-finish-command] Finish this tutorial and merge changes to the default branch: `wd finish`
~/next-big-app (master)> wd start setting-up-new-project
Started:
[setting-up-new-project] [P1] Things to do to set up your WHATDO.yaml for a project
~/next-big-app (setting-up-new-project)> wd start
```

# Lifecycle of a typical whatdo

```
~ (master)> wd start create-trash-button
# Branch 'create-trash-button' is created and pushed upstream

~ (create-trash-button)> git commit -A -m "Do the work involved with 'create-trash-button'"
~ (create-trash-button)> wd finish
# Delete 'create-trash-button' from WHATDO.yaml, commit the change, push 'create-trash-button',
# merge 'create-trash-button' into the default branch and push that as well

~ (master)> echo "Done with that task"
~ (master)> wd next
[implement-trash-buttton] Make it so when you drag an item onto the trash button it deletes it
```
