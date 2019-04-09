use crate::config::Config;
use crate::error::ErrorKind;
use console::Term;
use failure::{bail, Error, ResultExt};
use git2::{BranchType, Delta, ObjectType, Oid, Repository};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::PathBuf;
use structopt::{clap, StructOpt};
use tempfile::TempDir;

mod config;
mod error;
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
    let tgt = Repository::discover(".").context(ErrorKind::RepoDiscover)?;

    Config::check(&tgt)?;

    let (src, _dir) =
        setup_src(url, branch, tag).context(ErrorKind::RepoClone(String::from(url)))?;
    let commit = src.head()?.peel_to_commit()?;
    let config = Config::new(url, branch, tag, &commit);

    let src_ignore = get_ignore(&src)?;
    let tgt_ignore = get_ignore(&tgt)?;

    println!("Detect changes");
    init(&src, &tgt, &src_ignore, &tgt_ignore, force, true)?;
    println!("Apply changes");
    init(&src, &tgt, &src_ignore, &tgt_ignore, force, false)?;

    config.save(&tgt)?;

    Ok(())
}

fn cmd_update(force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".").context(ErrorKind::RepoDiscover)?;
    let mut config = Config::load(&tgt)?;

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())
        .context(ErrorKind::RepoClone(String::from(config.url.as_ref())))?;

    let src_ignore = get_ignore(&src)?;
    let tgt_ignore = get_ignore(&tgt)?;

    println!("Detect changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        true,
    )?;
    println!("Apply changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        false,
    )?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt)?;

    Ok(())
}

fn cmd_branch(branch: &str, force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = Config::load(&tgt)?;
    config.set_branch(&branch);

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())
        .context(ErrorKind::RepoClone(String::from(config.url.as_ref())))?;

    let src_ignore = get_ignore(&src)?;
    let tgt_ignore = get_ignore(&tgt)?;

    println!("Detect changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        true,
    )?;
    println!("Apply changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        false,
    )?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt)?;

    Ok(())
}

fn cmd_tag(tag: &str, force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let mut config = Config::load(&tgt)?;
    config.set_tag(&tag);

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())
        .context(ErrorKind::RepoClone(String::from(config.url.as_ref())))?;

    let src_ignore = get_ignore(&src)?;
    let tgt_ignore = get_ignore(&tgt)?;

    println!("Detect changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        true,
    )?;
    println!("Apply changes");
    update(
        &mut config,
        &src,
        &tgt,
        &src_ignore,
        &tgt_ignore,
        force,
        false,
    )?;

    let commit = src.head()?.peel_to_commit()?;
    config.set_commit(&commit);

    config.save(&tgt)?;

    Ok(())
}

fn cmd_clean(force: bool) -> Result<(), Error> {
    let tgt = Repository::discover(".")?;
    let config = Config::load(&tgt)?;

    let (src, _dir) = setup_src(&config.url, config.branch.as_ref(), config.tag.as_ref())
        .context(ErrorKind::RepoClone(String::from(config.url.as_ref())))?;

    let src_ignore = get_ignore(&src)?;
    let tgt_ignore = get_ignore(&tgt)?;

    println!("Detect changes");
    clean(&src, &tgt, &src_ignore, &tgt_ignore, force, true)?;
    println!("Apply changes");
    clean(&src, &tgt, &src_ignore, &tgt_ignore, force, false)?;

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
            src.find_branch(&format!("origin/{}", branch.as_ref()), BranchType::Remote)
                .context(ErrorKind::BranchNotFound(String::from(branch.as_ref())))?
                .get()
                .peel_to_commit()?
        } else if let Some(tag) = tag {
            src.find_reference(&format!("refs/tags/{}", tag.as_ref()))
                .context(ErrorKind::TagNotFound(String::from(tag.as_ref())))?
                .peel_to_commit()?
        } else {
            src.head()?.peel_to_commit()?
        };

        src.checkout_tree(commit.as_object(), None)?;
        src.set_head_detached(commit.id())?;
    }

    Ok((src, dir))
}

fn get_ignore(repo: &Repository) -> Result<Gitignore, Error> {
    let root = PathBuf::from(repo.workdir().unwrap());
    let path = root.join(".gitskelignore");
    let mut builder = GitignoreBuilder::new(root);
    builder.add(path);
    builder.add_line(None, ".gitskelignore")?;
    Ok(builder.build()?)
}

fn init(
    src: &Repository,
    tgt: &Repository,
    src_ignore: &Gitignore,
    tgt_ignore: &Gitignore,
    force: bool,
    dry_run: bool,
) -> Result<(), Error> {
    let mut warn = false;
    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        warn |= file::copy(src, tgt, src_ignore, tgt_ignore, &path, dry_run)?;
    }

    if warn && !force {
        bail!(ErrorKind::AbortByExist);
    }

    Ok(())
}

fn update(
    config: &mut Config,
    src: &Repository,
    tgt: &Repository,
    src_ignore: &Gitignore,
    tgt_ignore: &Gitignore,
    force: bool,
    dry_run: bool,
) -> Result<(), Error> {
    let src_obj = src.head()?.peel(ObjectType::Any)?;
    let tgt_obj = src
        .find_object(Oid::from_str(&config.revision)?, None)
        .context(ErrorKind::RevisionNotFound(String::from(
            config.revision.as_ref(),
        )))?;

    let src_tree = src_obj.peel_to_tree()?;
    let tgt_tree = tgt_obj.peel_to_tree()?;

    let diff = src.diff_tree_to_tree(Some(&tgt_tree), Some(&src_tree), None)?;

    let mut warn = false;
    for d in diff.deltas() {
        let mut delete = None;

        match d.status() {
            Delta::Added => (),
            Delta::Deleted => {
                delete = Some(d.new_file().path().unwrap());
            }
            Delta::Modified => (),
            _ => {
                unimplemented!();
            }
        }

        if let Some(delete) = delete {
            warn |= file::delete(tgt, src_ignore, tgt_ignore, delete, dry_run)?;
        }
    }

    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        warn |= file::copy(src, tgt, src_ignore, tgt_ignore, &path, dry_run)?;
    }

    if warn && !force {
        bail!(ErrorKind::AbortByModified);
    }

    Ok(())
}

fn clean(
    src: &Repository,
    tgt: &Repository,
    src_ignore: &Gitignore,
    tgt_ignore: &Gitignore,
    force: bool,
    dry_run: bool,
) -> Result<(), Error> {
    let mut warn = false;
    for index in src.index()?.iter() {
        let path = PathBuf::from(&String::from_utf8(index.path)?);
        warn |= file::delete(tgt, src_ignore, tgt_ignore, &path, dry_run)?;
    }

    if warn && !force {
        bail!(ErrorKind::AbortByModified);
    }

    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

#[cfg_attr(tarpaulin, skip)]
fn main() {
    openssl_probe::init_ssl_cert_env_vars();

    let err = Term::stderr();
    let opt = Opt::from_args();

    if let Err(x) = run(opt) {
        let mut cause = x.iter_chain();
        let _ = err.write_line(&format!(
            "{} {}",
            console::style("Error:").red().bold(),
            cause.next().unwrap()
        ));

        for x in cause {
            let _ = err.write_line(&format!(
                "  {} {}",
                console::style("Caused by:").white().bold(),
                x
            ));
        }
        std::process::exit(1);
    }
}

fn run(opt: Opt) -> Result<(), Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let args = vec![
            "git-skel",
            "init",
            "https://github.com/dalance/git-skel-test.git",
        ];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_ok());
    }

    fn init_fail() {
        let args = vec![
            "git-skel",
            "init",
            "https://github.com/dalance/git-skel-test.git",
        ];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_err());
    }

    fn update() {
        let args = vec!["git-skel", "update", "--force"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_ok());
    }

    fn branch() {
        let args = vec!["git-skel", "branch", "b1", "--force"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_ok());
    }

    fn branch_fail() {
        let args = vec!["git-skel", "branch", "b1"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_err());
    }

    fn tag() {
        let args = vec!["git-skel", "tag", "t1", "--force"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_ok());
    }

    fn tag_fail() {
        let args = vec!["git-skel", "tag", "t1"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_err());
    }

    fn clean() {
        let args = vec!["git-skel", "clean", "--force"];
        let opt = Opt::from_iter(args.iter());
        let ret = run(opt);
        assert!(ret.is_ok());
    }

    #[test]
    fn test_sequence() {
        // normal
        init();
        update();
        branch();
        tag();
        clean();

        // init fail
        init();
        init_fail();
        clean();

        // branch fail
        init();
        branch_fail();
        clean();

        // tag fail
        init();
        tag_fail();
        clean();
    }

}
