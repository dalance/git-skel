use failure::{bail, Error};
use git2::{BranchType, Delta, ObjectType, Oid, Repository};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::{clap, StructOpt};
use tempfile::TempDir;

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
    },
    #[structopt(name = "branch", about = "Sets tracking tag")]
    #[structopt(raw(setting = "clap::AppSettings::ColoredHelp"))]
    Tag {
        #[structopt(name = "TAG")]
        tag: String,
    },
}

// ---------------------------------------------------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub branch: Option<String>,
    pub revision: String,
}

// ---------------------------------------------------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------------------------------------------------

fn setup_src(url: &str, branch: Option<&str>) -> Result<(Repository, TempDir), Error> {
    let dir = tempfile::tempdir()?;
    let src = Repository::clone(url, &dir)?;

    {
        let commit = if let Some(branch) = branch {
            src.find_branch(&format!("origin/{}", branch), BranchType::Remote)?
                .get()
                .peel_to_commit()?
        } else {
            src.head()?.peel_to_commit()?
        };

        src.checkout_tree(commit.as_object(), None)?;
    }

    Ok((src, dir))
}

fn init(url: &str, branch: Option<&str>) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;

    let (src, _dir) = setup_src(url, branch)?;

    let commit = src.head()?.peel_to_commit()?;

    let config = Config {
        url: String::from(url),
        branch: branch.map(|x| String::from(x)),
        revision: format!("{}", commit.id()),
    };

    init_files(&src, &tgt, true)?;
    init_files(&src, &tgt, false)?;

    set_config(&tgt, &config, true)?;

    Ok(())
}

fn init_files(src: &Repository, tgt: &Repository, dry_run: bool) -> Result<(), Error> {
    let src_root = PathBuf::from(src.workdir().unwrap());
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());

    if dry_run {
        println!("Following files will be created");
    }

    let mut exists = false;
    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        let src_path = src_root.join(&path);
        let tgt_path = tgt_root.join(&path);

        if dry_run {
            if tgt_path.exists() {
                println!("  ! {}", path.to_string_lossy());
                exists = true;
            } else {
                println!("  - {}", path.to_string_lossy());
            }
        } else {
            if let Some(parent) = tgt_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(&parent)?;
                }
            }
            fs::copy(src_path, tgt_path)?;
        }
    }

    if exists {
        bail!("Abort: some files ( marked by ! ) exist");
    }

    Ok(())
}

fn update() -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = get_config(&tgt)?;

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref().map(String::as_ref))?;

    update_files(&mut config, &src, &tgt, true)?;

    set_config(&tgt, &config, false)?;

    Ok(())
}

fn update_files(
    config: &mut Config,
    src: &Repository,
    tgt: &Repository,
    dry_run: bool,
) -> Result<(), Error> {
    let src_root = PathBuf::from(src.workdir().unwrap());
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());

    let src_obj = src.head()?.peel(ObjectType::Any)?;
    let tgt_obj = src.find_object(Oid::from_str(&config.revision)?, None)?;

    let src_tree = src_obj.peel_to_tree()?;
    let tgt_tree = tgt_obj.peel_to_tree()?;

    let diff = src.diff_tree_to_tree(Some(&tgt_tree), Some(&src_tree), None)?;

    for d in diff.deltas() {
        let mut copy = None;
        let mut delete = None;

        match d.status() {
            Delta::Added => {
                copy = Some(d.new_file().path().unwrap());
            }
            Delta::Deleted => {}
            Delta::Modified => {
                copy = Some(d.new_file().path().unwrap());
            }
            Delta::Renamed => {}
            Delta::Copied => {}
            Delta::Typechange => {}
            _ => (),
        }

        if dry_run {
            if let Some(copy) = copy {
                let status = tgt.status_file(copy);
                dbg!(status);
            }
            if let Some(delete) = delete {
                let status = tgt.status_file(delete);
                dbg!(status);
            }
        } else {
        }

        dbg!(d.nfiles());
        dbg!(d.status());
        dbg!(d.old_file().path());
        dbg!(d.new_file().path());
    }

    Ok(())
}

fn get_config(tgt: &Repository) -> Result<Config, Error> {
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let config_path = tgt_root.join(".gitskel.toml");

    let mut f = fs::File::open(&config_path)?;
    let mut s = String::new();
    let _ = f.read_to_string(&mut s);
    let config = toml::from_str(&s)?;

    Ok(config)
}

fn set_config(tgt: &Repository, config: &Config, check: bool) -> Result<(), Error> {
    let tgt_root = PathBuf::from(tgt.workdir().unwrap());
    let config_path = tgt_root.join(".gitskel.toml");

    if check && config_path.exists() {
        bail!("config exists: {:?}", config_path);
    } else {
        fs::write(config_path, toml::to_string(config)?)?;
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
        Opt::Init { url, branch, force } => init(&url, branch.as_ref().map(String::as_ref))?,
        Opt::Update { force } => update()?,
        Opt::Branch { branch } => (),
        Opt::Tag { tag } => (),
    }

    //let dir = tempfile::tempdir()?;
    //let dir = dbg!(dir);

    //let url = "../git-skel-test";
    //let src = Repository::clone(url, &dir)?;

    //let tgt = Repository::discover(".")?;

    //let head = src.revparse_single("HEAD")?;
    //let prev = src.revparse_single("HEAD^")?;
    //dbg!(head.id());
    //dbg!(prev.id());
    //let head_tree = head.peel_to_tree()?;
    //let prev_tree = prev.peel_to_tree()?;
    //let diff = src.diff_tree_to_tree(Some(&head_tree), Some(&prev_tree), None)?;

    //for d in diff.deltas() {
    //    //dbg!(d.nfiles());
    //    dbg!(d.status());
    //    //dbg!(d.old_file().path());
    //    //dbg!(d.new_file().path());
    //}

    ////init("../git-skel-test", None)?;
    //init("../git-skel-test", Some("b1"))?;

    Ok(())
}
