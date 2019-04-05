use crate::error::ErrorKind;
use failure::{bail, Error, ResultExt};
use git2::{Commit, Repository};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub revision: String,
}

impl Config {
    pub fn new<T: AsRef<str>>(url: T, branch: Option<T>, tag: Option<T>, commit: &Commit) -> Self {
        Config {
            url: String::from(url.as_ref()),
            branch: branch.map(|x| String::from(x.as_ref())),
            tag: tag.map(|x| String::from(x.as_ref())),
            revision: format!("{}", commit.id()),
        }
    }

    pub fn set_branch(&mut self, branch: &str) {
        self.branch = Some(String::from(branch));
        self.tag = None;
    }

    pub fn set_tag(&mut self, tag: &str) {
        self.branch = None;
        self.tag = Some(String::from(tag));
    }

    pub fn set_commit(&mut self, commit: &Commit) {
        self.revision = format!("{}", commit.id());
    }

    pub fn load(tgt: &Repository) -> Result<Config, Error> {
        let tgt_root = PathBuf::from(tgt.workdir().unwrap());
        let config_path = tgt_root.join(".gitskel.toml");

        fn load_file(path: &Path) -> Result<Config, Error> {
            let mut f = fs::File::open(path)?;
            let mut s = String::new();
            let _ = f.read_to_string(&mut s);
            let config = toml::from_str(&s)?;
            Ok(config)
        }

        let config = load_file(&config_path).context(ErrorKind::ConfigLoad(
            config_path.to_string_lossy().to_string(),
        ))?;
        Ok(config)
    }

    pub fn save(&self, tgt: &Repository) -> Result<(), Error> {
        let tgt_root = PathBuf::from(tgt.workdir().unwrap());
        let config_path = tgt_root.join(".gitskel.toml");

        fs::write(&config_path, toml::to_string(self)?).context(ErrorKind::ConfigSave(
            config_path.to_string_lossy().to_string(),
        ))?;

        Ok(())
    }

    pub fn delete(tgt: &Repository) -> Result<(), Error> {
        let tgt_root = PathBuf::from(tgt.workdir().unwrap());
        let config_path = tgt_root.join(".gitskel.toml");

        fs::remove_file(config_path)?;

        Ok(())
    }

    pub fn check(tgt: &Repository) -> Result<(), Error> {
        let tgt_root = PathBuf::from(tgt.workdir().unwrap());
        let config_path = tgt_root.join(".gitskel.toml");

        if config_path.exists() {
            bail!(ErrorKind::AbortByConfigExist(
                config_path.to_string_lossy().to_string()
            ));
        } else {
            Ok(())
        }
    }
}
