use failure::Error;
use git2::Repository;
use std::fs;
use std::os::unix;
use std::path::{Path, PathBuf};

pub fn copy(src: &Repository, tgt: &Repository, path: &Path, dry_run: bool) -> Result<bool, Error> {
    let src_root = PathBuf::from(src.workdir().unwrap());
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let src_path = src_root.join(&path);
    let tgt_path = tgt_root.join(&path);

    let mut warn = false;
    if dry_run {
        let status = tgt.status_file(&tgt_path);
        let indicator = if let Ok(status) = status {
            if status.is_empty() {
                " copy  "
            } else {
                warn = true;
                "!copy  "
            }
        } else {
            if tgt_path.exists() {
                warn = true;
                "!copy  "
            } else {
                " copy  "
            }
        };
        println!("  {}: {}", indicator, path.to_string_lossy());
    } else {
        if let Some(parent) = tgt_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(&parent)?;
            }
        }
        if fs::symlink_metadata(&src_path)?.file_type().is_symlink() {
            let link_path = fs::read_link(&src_path)?;
            unix::fs::symlink(&link_path, &tgt_path)?;
        } else {
            fs::copy(&src_path, &tgt_path)?;
        }
    }

    Ok(warn)
}

pub fn delete(tgt: &Repository, path: &Path, dry_run: bool) -> Result<bool, Error> {
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let tgt_path = tgt_root.join(&path);

    let mut warn = false;
    if dry_run {
        let status = tgt.status_file(&tgt_path);
        let indicator = if let Ok(status) = status {
            if status.is_empty() {
                " delete"
            } else {
                warn = true;
                "!delete"
            }
        } else {
            if tgt_path.exists() {
                warn = true;
                "!delete"
            } else {
                "missing"
            }
        };
        println!("  {}: {}", indicator, path.to_string_lossy());
    } else {
        remove_recursive(&tgt_path)?;
    }

    Ok(warn)
}

fn remove_recursive(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        fs::remove_dir(path)?;
    } else {
        fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        if fs::read_dir(parent)?.count() == 0 {
            remove_recursive(parent)?;
        }
    }

    Ok(())
}
