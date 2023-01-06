use cargo::core::source::GitReference;
use cargo::core::source::SourceId;
use cargo::sources::git::GitRemote;
use cargo::util::hex::short_hash;
use cargo::util::{CanonicalUrl, Config};
use cargo_toml::{Dependency, Manifest};
use clap::Parser;
use in_toto::crypto::PublicKey;
use in_toto::models::Metablock;
use in_toto::verifylib::in_toto_verify;
use log::{debug, error, info};
use serde_json;
use std::collections::HashMap;
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

    #[arg(
        short,
        long,
        help = "Project artifacts directory to use instead of ~/.cargo/git"
    )]
    project_dir: Option<PathBuf>,
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

struct InTotoVerify {}

pub fn copy_all(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_all(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

impl InTotoVerify {
    fn verify(artifact_dir: &PathBuf, dependency: &String) {
        debug!("artifact_dir: {:?}", &artifact_dir);
        let verify_dir = Path::new("verify_dir");
        copy_all(artifact_dir, verify_dir).unwrap();

        let layout_filename = format!("{}-layout.json", dependency);
        let layout_path = verify_dir.join(layout_filename);
        let layout_bytes = fs::read(layout_path).expect("read layout failed");
        let layout = serde_json::from_slice::<Metablock>(&layout_bytes)
            .expect("Could not deserialize Metablock");

        let pub_key_path = verify_dir.join("cosign.pub");
        let pub_key_pem = fs::read_to_string(pub_key_path).unwrap();
        let pub_key = PublicKey::from_pem_spki(
            &pub_key_pem,
            in_toto::crypto::SignatureScheme::EcdsaP256Sha256,
        )
        .unwrap();

        let key_id = layout.signatures[0].key_id().clone();
        debug!("PublicKey::key_id: {:?}", &pub_key.key_id());
        debug!("Layout:key_id: {:?}", &key_id);
        let layout_keys = HashMap::from([(key_id, pub_key)]);

        let current_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(verify_dir).unwrap();
        //in_toto_verify(&layout, layout_keys, verify_dir.to_str().unwrap(), None)
        in_toto_verify(&layout, layout_keys, ".", None).expect("verify failed");

        info!("Verification succeeded!");
        std::env::set_current_dir(current_dir).unwrap();
        fs::remove_dir_all(verify_dir).unwrap();
    }
}

fn verify_cargo_artifact(
    src_dir: &PathBuf,
    artifacts_path: &str,
    _artifact_name: &str,
    dependency_name: &str,
) {
    let artifacts_dir = src_dir.join(artifacts_path);
    if !artifacts_dir.exists() {
        error!(
            "Could not perform verification of dependency '{}', an artifacts \
             directory named '{}' could not be found in '{}'\n",
            &dependency_name,
            &artifacts_path,
            &src_dir.display()
        );
        std::process::exit(1);
    }
    InTotoVerify::verify(&artifacts_dir, &dependency_name.to_string());
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let dependency_name = args.dependency;
    let config = Config::default().unwrap();
    let cargo_home = home::cargo_home().expect("Could not find the cargo home directory");

    info!("Verifying {}", &dependency_name);

    let manifest_file = fs::read(&args.manifest_path).unwrap();
    let manifest = Manifest::from_slice(&manifest_file).unwrap();
    match manifest.dependencies.get(&dependency_name) {
        Some(Dependency::Simple(version)) => {
            // This means that it is a crates.io dep and will be in
            // .cargo/registry/src directory (I think).
            // In this case we only have the dependency name and its version.
            let registry_id = SourceId::crates_io(&config).unwrap();
            let host = registry_id.url().host().unwrap().to_string();
            let dir_name = format!("{}-{}", host, short_hash(&registry_id));
            let src_dir = cargo_home.join("registry").join("src").join(dir_name);
            let dep_dir = src_dir.join(format!("{}-{}", dependency_name, version));
            if !dep_dir.exists() {
                error!("The dependency {} could not be found", dependency_name);
                std::process::exit(1);
            }
            verify_cargo_artifact(&dep_dir, &args.artifacts_path, &version, &dependency_name)
        }
        Some(Dependency::Detailed(detail)) => {
            if detail.git.is_some() {
                let cargo_git =
                    CargoGit::new(detail.git.as_ref().unwrap(), &dependency_name, &cargo_home);

                let main = String::from("main");
                if detail.branch.is_some() {
                    let branch = detail.branch.as_ref().unwrap_or(&main);
                    let dep_dir = if args.project_dir.is_some() {
                        args.project_dir.unwrap()
                    } else {
                        cargo_git.rev_directory(branch)
                    };
                    verify_cargo_artifact(&dep_dir, &args.artifacts_path, branch, &dependency_name);
                }
                if detail.tag.is_some() {
                    unimplemented!("Tags are currently not supported");
                }
                if detail.rev.is_some() {
                    unimplemented!("Revisions are currently not supported");
                }
            } else {
                error!("version: {}", &detail.version.as_ref().unwrap());
                unimplemented!("crates.io deps are currently not supported");
            }
        }
        Some(Dependency::Inherited(detail)) => {
            error!("Inherited dep: {:?}", detail);
            unimplemented!("Inherited deps are currently not supported");
        }
        None => {
            error!("Could not find the dependency: {dependency_name} in Cargo.toml");
            std::process::exit(1);
        }
    }
}
