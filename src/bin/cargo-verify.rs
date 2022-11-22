use cargo::core::source::GitReference;
use cargo::sources::git::GitRemote;
use cargo::util::hex::short_hash;
use cargo::util::CanonicalUrl;
use cargo_toml::{Dependency, Manifest};
use clap::Parser;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Parser, Debug)]
#[command(author,
    version,
    long_about = None)]
/// cargo-verify is a tool that verifies a project somehow...
struct Args {
    #[arg(
        short,
        long,
        help = "Path to Cargo.toml file to use",
        default_value = "Cargo.toml"
    )]
    manifest_path: String,

    #[arg(short, long, help = "The dependency to verify")]
    dependency: String,

    #[arg(
        short,
        long,
        help = "The path to the artifacts directory in the project to verify",
        default_value = "sscs/in-toto/artifacts"
    )]
    artifacts_path: String,
}

struct CargoGit {
    url: Url,
    db_path: Box<PathBuf>,
    checkouts_path: Box<PathBuf>,
}

impl CargoGit {
    fn new(repo_url: &str, dependency_name: &String, cargo_home: &Path) -> Self {
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

    fn rev_directory(&self, branch: &String) -> PathBuf {
        let git_ref = GitReference::Branch(branch.to_string());
        let git_remote = GitRemote::new(&self.url);
        let oid = git_remote.rev_for(&self.db_path, &git_ref).unwrap();
        let short = &oid.to_string()[..7];
        println!("Branch: {} resolved to revision {}", &branch, short);
        self.checkouts_path.join(short)
    }
}

impl fmt::Display for CargoGit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "db_path: {}", &self.db_path.display())?;
        write!(f, "checkouts_path: {}", &self.checkouts_path.display())
    }
}

fn main() {
    let args = Args::parse();
    let dependency_name = args.dependency;
    println!("Verifying dependency: {}", dependency_name);

    let cargo_home = home::cargo_home().expect("Could not find the cargo home directory");

    let manifest_file = fs::read(&args.manifest_path).unwrap();
    let manifest = Manifest::from_slice(&manifest_file).unwrap();
    let dependency = manifest
        .dependencies
        .get(&dependency_name)
        .expect("Could not find the dependency: {}");
    match dependency {
        Dependency::Simple(version) => {
            // This means that it is a crates.io dep and will be in
            // .cargo/registry/src directory (I think).
            println!("Simple dep version: {}", version);
            unimplemented!("Simple deps are currently not supported");
        }
        Dependency::Detailed(detail) => {
            println!("Detailed dep: {:?}", &detail);
            if detail.git.is_some() {
                let cargo_git =
                    CargoGit::new(detail.git.as_ref().unwrap(), &dependency_name, &cargo_home);
                println!("{}", cargo_git);

                let main = String::from("main");
                if detail.branch.is_some() {
                    let branch = detail.branch.as_ref().unwrap_or(&main);
                    let checkout_dir = cargo_git.rev_directory(branch);
                    let artifacts_dir = checkout_dir.join(&args.artifacts_path);

                    //println!("artifacts_dir: {:?}", &artifacts_dir);
                    if !artifacts_dir.exists() {
                        eprintln!(
                            "Could not perform verification as the artifacts \
                             directory named '{}' could not be found in\n'{}'",
                            &args.artifacts_path,
                            &checkout_dir.display()
                        );
                        std::process::exit(1);
                    }
                }
                if detail.tag.is_some() {
                    unimplemented!("Tag are currently not supported");
                }
                if detail.rev.is_some() {
                    unimplemented!("Revisions are currently not supported");
                }
            } else {
                println!("version: {}", &detail.version.as_ref().unwrap());
                unimplemented!("crates.io deps are currently not supported");
            }
        }
        Dependency::Inherited(detail) => {
            println!("Inherited dep: {:?}", detail);
            unimplemented!("Inherited deps are currently not supported");
        }
    }
}
