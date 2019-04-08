use failure::Error;
use git2::Repository;
use ignore::gitignore::Gitignore;
use std::fs;
use std::io::Read;
use std::os::unix;
use std::path::{Path, PathBuf};

pub fn copy(
    src: &Repository,
    tgt: &Repository,
    src_ignore: &Gitignore,
    tgt_ignore: &Gitignore,
    path: &Path,
    dry_run: bool,
) -> Result<bool, Error> {
    let src_root = PathBuf::from(src.workdir().unwrap());
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let src_path = src_root.join(&path);
    let tgt_path = tgt_root.join(&path);

    let src_ignored = is_ignore(src_ignore, path);
    let tgt_ignored = is_ignore(tgt_ignore, path);
    let ignored = src_ignored || tgt_ignored;

    let mut warn = false;
    if is_diff(&src_path, &tgt_path)? {
        if dry_run {
            let status = tgt.status_file(&tgt_path);
            let indicator = if ignored {
                " ignore"
            } else if let Ok(status) = status {
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
            if !ignored {
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
        }
    }

    Ok(warn)
}

fn is_diff(src_path: &Path, tgt_path: &Path) -> Result<bool, Error> {
    if let Ok(mut src) = fs::File::open(src_path) {
        if let Ok(mut tgt) = fs::File::open(tgt_path) {
            let mut src_buf = Vec::new();
            let mut tgt_buf = Vec::new();

            src.read_to_end(&mut src_buf)?;
            tgt.read_to_end(&mut tgt_buf)?;

            Ok(src_buf != tgt_buf)
        } else {
            Ok(true)
        }
    } else {
        Ok(true)
    }
}

fn is_ignore(ignore: &Gitignore, path: &Path) -> bool {
    ignore.matched(path, false).is_ignore()
}

pub fn delete(
    tgt: &Repository,
    tgt_ignore: &Gitignore,
    path: &Path,
    dry_run: bool,
) -> Result<bool, Error> {
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let tgt_path = tgt_root.join(&path);
    let ignored = is_ignore(tgt_ignore, path);

    let mut warn = false;
    if dry_run {
        let status = tgt.status_file(&tgt_path);
        let indicator = if ignored {
            " ignore"
        } else if let Ok(status) = status {
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
        if !ignored && tgt_path.exists() {
            remove_recursive(&tgt_path)?;
        }
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
