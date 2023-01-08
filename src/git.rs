use anyhow::anyhow;
use cargo::core::source::GitReference;
use cargo::sources::git::GitRemote;
use cargo::util::hex::short_hash;
use cargo::util::CanonicalUrl;
use log::debug;
use std::fmt;
use std::path::{Path, PathBuf};
use url::Url;

pub struct CargoGit {
    url: Url,
    db_path: Box<PathBuf>,
    checkouts_path: Box<PathBuf>,
}

impl CargoGit {
    pub fn new(repo_url: &str, dependency_name: &String, cargo_home: &Path) -> Self {
        let url = Url::parse(repo_url).unwrap();
        let can_url = CanonicalUrl::new(&url).unwrap();
        let repo_name = format!("{}-{}", dependency_name, short_hash(&can_url));
        let db_path = cargo_home.join("git").join("db").join(&repo_name);
        let checkouts_path = cargo_home.join("git").join("checkouts").join(&repo_name);
        Self {
            url,
            db_path: Box::new(db_path),
            checkouts_path: Box::new(checkouts_path),
        }
    }

    pub fn rev_directory(&self, branch: &String) -> PathBuf {
        let git_ref = GitReference::Branch(branch.to_string());
        let git_remote = GitRemote::new(&self.url);
        let oid = git_remote.rev_for(&self.db_path, &git_ref).unwrap();
        let short = &oid.to_string()[..7];
        debug!("Branch: {} resolved to revision {}\n", &branch, short);
        self.checkouts_path.join(short)
    }
}

impl fmt::Display for CargoGit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "git_db_path: {}", &self.db_path.display())?;
        write!(f, "git_checkouts_path: {}", &self.checkouts_path.display())
    }
}

pub fn get_github_org_and_name<'a>(url: &'a str) -> anyhow::Result<(&'a str, &'a str)> {
    if url.starts_with("git") {
        let org_and_repo = url.split(":").nth(1).unwrap();
        let org_and_repo = org_and_repo.split(".").nth(0).unwrap();
        let org = org_and_repo.split("/").nth(0).unwrap();
        let repo = org_and_repo.split("/").nth(1).unwrap();
        Ok((org, repo))
    } else if url.starts_with("https") {
        Err(anyhow!("github https protocol is not supported yet"))
    } else {
        Err(anyhow!("unknown protocol"))
    }
}
