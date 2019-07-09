use failure::Fail;

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "failed to discover current repository")]
    RepoDiscover,
    #[fail(display = "failed to clone target repository: {}", 0)]
    RepoClone(String),
    #[fail(display = "failed to find branch: {}", 0)]
    BranchNotFound(String),
    #[fail(display = "failed to find tag: {}", 0)]
    TagNotFound(String),
    #[fail(display = "failed to find revision: {}", 0)]
    RevisionNotFound(String),
    #[fail(display = "failed to load config: {}", 0)]
    ConfigLoad(String),
    #[fail(display = "failed to save config: {}", 0)]
    ConfigSave(String),
    #[fail(
        display = "aborted bacause\n         - some files are not committed    ( marked by ! )\n         - some files are modified locally ( marked by * )\n       If you will ignore it, use `--force` option."
    )]
    AbortByModified,
    #[fail(
        display = "aborted bacause some files exist ( marked by ! )\n       If you will ignore it, use `--force` option."
    )]
    AbortByExist,
    #[fail(display = "aborted bacause config file exists: {}", 0)]
    AbortByConfigExist(String),
}
