use clap::Parser;
use in_toto::runlib;
use source_distributed::private_key_from_file;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author,
    version,
    long_about = None)]
/// clone-project step...
struct Args {
    #[arg(
        short,
        long,
        help = "The github organisation that the project/repository belongs to"
    )]
    org_name: String,

    #[arg(short, long, help = "The github repository/project")]
    repo_name: String,

    #[arg(long, help = "The private key to be used to sign the layout")]
    private_key: PathBuf,

    #[arg(
        short,
        long,
        help = "The name of the step",
        default_value = "clone-project"
    )]
    step_name: String,

    #[arg(
        long,
        help = "The directory to where the steps will be performed",
        default_value = "sscs/in-toto/work"
    )]
    work_dir: PathBuf,

    #[arg(
        long,
        help = "The directory to store the artifacts in.",
        default_value = "sscs/in-toto/artifacts"
    )]
    artifacts_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let org_name = args.org_name;
    let repo_name = args.repo_name;
    println!("Generate in-toto step for {}/{}", org_name, repo_name);

    let work_dir = &args.work_dir;
    if !work_dir.exists() {
        println!("Creating work dir {:?}", &work_dir);
        fs::create_dir(&work_dir)
            .expect(format!("Could not create working directory {:?}", &work_dir).as_str());
    }

    let priv_key = private_key_from_file(&args.private_key);
    println!("key_id: {:?}", priv_key.key_id().prefix());

    let link = runlib::in_toto_run(
        &args.step_name,                   // name
        Some(&work_dir.to_str().unwrap()), // workdir
        &[work_dir.to_str().unwrap()],     // materials
        &[
            // products
            "Cargo.toml",
            "Cargo.lock",
            "README.md",
            "src",
        ],
        &[
            "git",
            "clone",
            format!("git@github.com:{}/{}.git", org_name, repo_name).as_str(),
        ],
        Some(&priv_key),
        Some(&["sha512", "sha256"]),
        None,
    )
    .unwrap();
    let json = serde_json::to_value(&link).unwrap();

    let filename = format!("{}.{}.link", args.step_name, priv_key.key_id().prefix());
    let path = &args.artifacts_dir.join(&filename);
    let s = serde_json::to_string_pretty(&json).unwrap();
    fs::write(&path, s).unwrap();
    println!("Generated {}", path.display());
}
