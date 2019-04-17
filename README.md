# git-skel

**git-skel** is a git subcommand to apply skeleton repository continuously.

[![Build Status](https://dev.azure.com/dalance/git-skel/_apis/build/status/dalance.git-skel?branchName=master)](https://dev.azure.com/dalance/git-skel/_build/latest?definitionId=1&branchName=master)
[![Crates.io](https://img.shields.io/crates/v/git-skel.svg)](https://crates.io/crates/git-skel)
[![codecov](https://codecov.io/gh/dalance/git-skel/branch/master/graph/badge.svg)](https://codecov.io/gh/dalance/git-skel)

## Description

Skeleton repository is a project templete including trivial directories, scripts, configs, and so on.
(You can find many skeleton repositories by searching `skeleton` in GitHub.)

Usually skeleton repository is used at the initial phase of project by cloning the repository.
If the skeleton repository is updated after the project is grown, the way to apply the update is carefully file copy or `git cherry-pick`.
Both of them are not easy.

**git-skel** provides the easy way to apply the update.

## Platform

Linux/macOS/Windows

## Installation

### Download binary

Download from [release page](https://github.com/dalance/git-skel/releases/latest), and extract to the directory in PATH.

### Cargo

You can install by [cargo](https://crates.io).

```
cargo install git-skel
```

## Demo

[![asciicast](https://asciinema.org/a/241332.svg)](https://asciinema.org/a/241332?autoplay=1&speed=1.5)

## Usage

### Init

Initially you can setup to apply a skeleton repository in any git repository like below:

```
$ git skel init [URL]
```

`git skel init` command clones `[URL]` to a temporary directory and copies all files to the current repository.
The command puts `.gitskel.toml` to the current repository to record the path and revision of the skeleton repository.
You can check the added files by `git status` and commit if there is no problem.

### Update

If the skeleton repository is updated, you can apply the update like below:

```
$ git skel update
```

`git skel update` command clones the skeleton repository saved in `.gitskel.toml` to a temporary directory and copies all files to the current repository.
If there are deleted files between the latest revision and the saved revision in `.gitskel.toml`, the files will be deleted.
If the files which will be changed by the command are modified and not committed, the command will be aborted.

```
$ git skel update
Detect changes
  !copy  : aaa
Error: aborted bacause some files are not committed ( marked by ! )
       If you will ignore it, use `--force` option.
```

You can ignore this check by `git skel update --force`.

### Branch / Tag

`git skel branch` command change the branch to track and update.

```
$ git skel branch [BRANCK NAME]
```

`git skel tag` command change the tag to track and update.

```
$ git skel tag [TAG NAME]
```

`--force` option can be used as the same as update.

### Clean

`git skel clean` command delete `.gitskel.toml` and all files which copied from the skeleton repository.

```
$ git skel clean
```

`--force` option can be used as the same as update.

### `.gitskelignore`

You can put `.gitskelignore` to repository root.
This has the same syntax as `.gitignore`.
Any file matched with `.gitskelignore` is ignored by the command.

`.gitskelignore` can be used at both skeleton repository and project repository.
For example, `README.md` should be added to `.gitskelignore` of a skeleton repository because `README.md` shoud not be copied to a project repository.
If there are the files modified by project-specific reason, the files should be added to `.gitskelignore` of a project repository.
