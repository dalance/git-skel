use crate::config::Config;
use failure::{bail, Error};
use git2::{BranchType, Delta, ObjectType, Oid, Repository};
use std::path::PathBuf;
use structopt::{clap, StructOpt};
use tempfile::TempDir;

mod config;
mod file;

// ---------------------------------------------------------------------------------------------------------------------
// Opt
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, StructOpt)]
#[structopt(raw(
    long_version = "option_env!(\"LONG_VERSION\").unwrap_or(env!(\"CARGO_PKG_VERSION\"))"
))]
#[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
pub enum Opt {
    #[structopt(name = "init", about = "Initializes")]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Init {
        #[structopt(name = "URL")]
        url: String,
        #[structopt(short = "b", long = "branch")]
        branch: Option<String>,
        #[structopt(short = "t", long = "tag")]
        tag: Option<String>,
        #[structopt(short = "f", long = "force")]
        force: bool,
    },
    #[structopt(
        name = "update",
        about = "Updates to the latest revision of the upstream repository"
    )]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Update {
        #[structopt(short = "f", long = "force")]
        force: bool,
    },
    #[structopt(name = "branch", about = "Sets tracking branck")]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Branch {
        #[structopt(name = "BRANCH")]
        branch: String,
        #[structopt(short = "f", long = "force")]
        force: bool,
    },
    #[structopt(name = "tag", about = "Sets tracking tag")]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Tag {
        #[structopt(name = "TAG")]
        tag: String,
        #[structopt(short = "f", long = "force")]
        force: bool,
    },
    #[structopt(name = "clean", about = "Removes skeleton files")]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Clean {
        #[structopt(short = "f", long = "force")]
        force: bool,
    },
}

// ---------------------------------------------------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------------------------------------------------

fn cmd_init(url: &str, branch: Option<&str>, tag: Option<&str>, force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let (src, _dir) = setup_src(url, branch, tag)?;
    let commit = src.head()?.peel_to_commit()?;
    let config = Config::new(url, branch, tag, &commit);

    init(&src, &tgt, force, true)?;
    init(&src, &tgt, force, false)?;

    config.save(&tgt, true)?;

    Ok(())
}

fn cmd_update(force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = Config::load(&tgt)?;

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())?;

    update(&mut config, &src, &tgt, force, true)?;
    update(&mut config, &src, &tgt, force, false)?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt, false)?;

    Ok(())
}

fn cmd_branch(branch: &str, force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = Config::load(&tgt)?;
    config.set_branch(&branch);

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())?;

    update(&mut config, &src, &tgt, force, true)?;
    update(&mut config, &src, &tgt, force, false)?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt, false)?;

    Ok(())
}

fn cmd_tag(tag: &str, force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = Config::load(&tgt)?;
    config.set_tag(&tag);

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())?;

    update(&mut config, &src, &tgt, force, true)?;
    update(&mut config, &src, &tgt, force, false)?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt, false)?;

    Ok(())
}

fn cmd_clean(force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let config = Config::load(&tgt)?;

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())?;

    clean(&src, &tgt, force, true)?;
    clean(&src, &tgt, force, false)?;

    Config::delete(&tgt)?;

    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Support functions
// ---------------------------------------------------------------------------------------------------------------------

fn setup_src<T: AsRef<str>>(
    url: T,
    branch: Option<T>,
    tag: Option<T>,
) -> Result<(Repository, TempDir), Error> {
    let dir = tempfile::tempdir()?;
    let src = Repository::clone(url.as_ref(), &dir)?;

    {
        let commit = if let Some(branch) = branch {
            src.find_branch(&format!("origin/{}", branch.as_ref()), BranchType::Remote)?
                .get()
                .peel_to_commit()?
        } else if let Some(tag) = tag {
            src.find_reference(&format!("refs/tags/{}", tag.as_ref()))?
                .peel_to_commit()?
        } else {
            src.head()?.peel_to_commit()?
        };

        src.checkout_tree(commit.as_object(), None)?;
        src.set_head_detached(commit.id())?;
    }

    Ok((src, dir))
}

fn init(src: &Repository, tgt: &Repository, force: bool, dry_run: bool) -> Result<(), Error> {
    let mut warn = false;
    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        warn |= file::copy(src, tgt, &path, dry_run)?;
    }

    if warn && !force {
        bail!("Abort: some files ( marked by ! ) exist");
    }

    Ok(())
}

fn update(
    config: &mut Config,
    src: &Repository,
    tgt: &Repository,
    force: bool,
    dry_run: bool,
) -> Result<(), Error> {
    let src_obj = src.head()?.peel(ObjectType::Any)?;
    let tgt_obj = src.find_object(Oid::from_str(&config.revision)?, None)?;

    let src_tree = src_obj.peel_to_tree()?;
    let tgt_tree = tgt_obj.peel_to_tree()?;

    let diff = src.diff_tree_to_tree(Some(&tgt_tree), Some(&src_tree), None)?;

    let mut warn = false;
    for d in diff.deltas() {
        let mut copy = None;
        let mut delete = None;

        dbg!(d.status());
        match d.status() {
            Delta::Added => {
                copy = Some(d.new_file().path().unwrap());
            }
            Delta::Deleted => {
                delete = Some(d.new_file().path().unwrap());
            }
            Delta::Modified => {
                copy = Some(d.new_file().path().unwrap());
            }
            _ => {
                unimplemented!();
            }
        }

        if let Some(copy) = copy {
            warn |= file::copy(src, tgt, copy, dry_run)?;
        }
        if let Some(delete) = delete {
            dbg!(delete);
            warn |= file::delete(tgt, delete, dry_run)?;
        }
    }

    if warn && !force {
        bail!("Abort: some files ( marked by ! ) are not committed");
    }

    Ok(())
}

fn clean(src: &Repository, tgt: &Repository, force: bool, dry_run: bool) -> Result<(), Error> {
    let mut warn = false;
    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        warn |= file::delete(tgt, &path, dry_run)?;
    }

    if warn && !force {
        bail!("Abort: some files ( marked by ! ) are not committed");
    }

    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

#[cfg_attr(tarpaulin, skip)]
fn main() {
    if let Err(x) = run() {
        eprintln!("{:?}", x);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let opt = Opt::from_args();

    match opt {
        Opt::Init {
            url,
            branch,
            tag,
            force,
        } => cmd_init(
            &url,
            branch.as_ref().map(String::as_ref),
            tag.as_ref().map(String::as_ref),
            force,
        )?,
        Opt::Update { force } => cmd_update(force)?,
        Opt::Branch { branch, force } => cmd_branch(&branch, force)?,
        Opt::Tag { tag, force } => cmd_tag(&tag, force)?,
        Opt::Clean { force } => cmd_clean(force)?,
    }

    Ok(())
}
