use cargo::core::source::GitReference;
use cargo::sources::git::GitRemote;
use cargo::util::hex::short_hash;
use cargo::util::CanonicalUrl;
use cargo_toml::{Dependency, Manifest};
use clap::Parser;
use std::fmt;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
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
        println!("Branch: {} resolved to revision {}\n", &branch, short);
        self.checkouts_path.join(short)
    }
}

impl fmt::Display for CargoGit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "git_db_path: {}", &self.db_path.display())?;
        write!(f, "git_checkouts_path: {}", &self.checkouts_path.display())
    }
}

struct InTotoVerify {}

impl InTotoVerify {
    fn verify(artifact_tar: PathBuf) {
        let tar = File::open(artifact_tar).unwrap();
        let mut archive = Archive::new(tar);
        let verify_dir: &'static str = "verify_dir";
        fs::create_dir(verify_dir).unwrap();
        archive.unpack(verify_dir).unwrap();

        let output = Command::new("in-toto-verify")
            .current_dir(verify_dir)
            .arg("-v")
            .arg("--key-types")
            .arg("ecdsa")
            .arg("--layout")
            .arg("source-distributed-layout.json")
            .arg("--layout-keys")
            .arg("cosign.key.pub.json")
            .output()
            .expect("failed to execute in-toto");
        println!("verify status: {}", output.status);
        println!("verify stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("verify stderr: {}", String::from_utf8_lossy(&output.stderr));

        fs::remove_dir_all(verify_dir).unwrap();
    }
}

fn main() {
    let args = Args::parse();
    let dependency_name = args.dependency;
    println!("Verifying dependency: {}\n", dependency_name);

    let cargo_home = home::cargo_home().expect("Could not find the cargo home directory");

    let manifest_file = fs::read(&args.manifest_path).unwrap();
    let manifest = Manifest::from_slice(&manifest_file).unwrap();
    let dependency = manifest
        .dependencies
        .get(&dependency_name)
        .expect("Could not find the dependency: {dependency_name}");
    match dependency {
        Dependency::Simple(version) => {
            // This means that it is a crates.io dep and will be in
            // .cargo/registry/src directory (I think).
            println!("Simple dep version: {}", version);
            unimplemented!("Simple deps are currently not supported");
        }
        Dependency::Detailed(detail) => {
            //println!("Detailed dep: {:?}", &detail);
            if detail.git.is_some() {
                let cargo_git =
                    CargoGit::new(detail.git.as_ref().unwrap(), &dependency_name, &cargo_home);
                println!("{}\n", cargo_git);

                let main = String::from("main");
                if detail.branch.is_some() {
                    let branch = detail.branch.as_ref().unwrap_or(&main);
                    let checkout_dir = cargo_git.rev_directory(branch);
                    let artifacts_dir = checkout_dir.join(&args.artifacts_path);

                    println!("artifacts_dir: {:?}", &artifacts_dir);
                    if !artifacts_dir.exists() {
                        eprintln!(
                            "Could not perform verification as the artifacts \
                             directory named '{}' could not be found in\n'{}'",
                            &args.artifacts_path,
                            &checkout_dir.display()
                        );
                        std::process::exit(1);
                    }
                    let artifact_tar = artifacts_dir.join(format!("{branch}.tar"));
                    println!("artifact_tar: {:?}\n", artifact_tar);

                    if !artifact_tar.exists() {
                        eprintln!(
                            "Could not perform verification as the artifact \
                             tar named '{}' could not be found in\n'{}'",
                            &artifact_tar.display(),
                            &artifacts_dir.display()
                        );
                        std::process::exit(1);
                    }
                    InTotoVerify::verify(artifact_tar);
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
