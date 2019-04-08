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

## Installation

### Download binary

Download from [release page](https://github.com/dalance/git-skel/releases/latest), and extract to the directory in PATH.

### Cargo

You can install by [cargo](https://crates.io).

```
cargo install git-skel
```

## Usage

