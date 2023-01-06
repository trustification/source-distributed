use cargo::util::Config;
use clap::Parser;
use git2::Repository;
use log::{debug, error};
use source_distributed::steps::{
    clone_project, run_tests, write_layout_to_file, write_step_to_file,
};
use source_distributed::{
    create_layout, generate_keypair, get_github_org_and_name, priv_key_from_pem,
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author,
    version,
    long_about = None)]
/// create-layout generates an in-toto layout.json.
struct Args {
    #[arg(
        short,
        long,
        help = "The directory to store the artifacts in.",
        default_value = "sscs/in-toto/artifacts"
    )]
    artifacts_dir: PathBuf,

    #[arg(
        short,
        long,
        help = "The number of days that the layout should be valid",
        default_value = "365"
    )]
    valid_days: u64,

    #[arg(
        short,
        long,
        help = "An optional provider token, for example a github access token"
    )]
    provider_token: Option<String>,

    #[arg(
        short,
        long,
        help = "The directory to where the steps will be performed",
        default_value = "sscs/in-toto/work"
    )]
    work_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    let _config = Config::default().unwrap();
    let _cargo_home = home::cargo_home().expect("Could not find the cargo home directory");

    let repository = Repository::discover(".").unwrap();
    let head = repository.head().unwrap();
    let branch = head.shorthand().unwrap();
    let _commit = head.peel_to_commit().unwrap().id();

    let remotes = repository.remotes().unwrap();
    let remote = repository.find_remote(remotes.get(0).unwrap()).unwrap();
    let url = remote.url().unwrap();
    let (org_name, repo_name) = get_github_org_and_name(url).unwrap();

    let branch_dir = &args.artifacts_dir.join(branch);
    let work_dir = &branch_dir.join("work");

    if !branch_dir.exists() {
        fs::create_dir(&branch_dir)
            .expect(format!("Could not create branch_dir {:?}", &branch_dir).as_str());
    }

    if let Ok(keypair) = generate_keypair(args.provider_token).await {
        debug!(
            "Generated keypair {:?}",
            &keypair.private_key_to_pem().unwrap()
        );
        let priv_key = keypair.private_key_to_pem().unwrap();
        let priv_key = priv_key_from_pem(&priv_key).unwrap();

        let public_key_pem = keypair.public_key_to_pem().unwrap();
        let public_key_path = branch_dir.join("cosign.pub");
        fs::write(public_key_path, public_key_pem).expect("Could not write public key");

        // Generate layout
        let layout = create_layout(&org_name, &repo_name, &priv_key, args.valid_days).unwrap();
        write_layout_to_file(&layout, repo_name, &branch_dir).unwrap();

        // Create work dir for steps
        if !work_dir.exists() {
            fs::create_dir(&work_dir)
                .expect(format!("Could not create work directory {:?}", &work_dir).as_str());
        }

        // Generate clone-project step
        let clone_project =
            clone_project(org_name, repo_name, &priv_key, work_dir.to_path_buf()).unwrap();
        write_step_to_file(&clone_project, "clone-project", &priv_key, branch_dir).unwrap();

        // Generate run-tests step
        let run_tests = run_tests(&priv_key, work_dir).unwrap();
        write_step_to_file(&run_tests, "run-tests", &priv_key, branch_dir).unwrap();

        fs::remove_dir_all(&work_dir).unwrap();
    } else {
        error!("Could not generate keypair");
    }
}
