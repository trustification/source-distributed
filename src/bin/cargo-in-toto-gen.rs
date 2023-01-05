use cargo::util::Config;
use clap::Parser;
use git2::Repository;
use in_toto::runlib;
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
    let args = Args::parse();
    let _config = Config::default().unwrap();
    let _cargo_home = home::cargo_home().expect("Could not find the cargo home directory");

    let repository = Repository::discover(".").unwrap();
    let head = repository.head().unwrap();
    let branch = head.shorthand().unwrap();
    let commit = head.peel_to_commit().unwrap().id();

    let remotes = repository.remotes().unwrap();
    println!("remotes: {:?}", &remotes.get(0).unwrap());
    let remote = repository.find_remote(remotes.get(0).unwrap()).unwrap();
    let url = remote.url().unwrap();
    let (org_name, repo_name) = get_github_org_and_name(url).unwrap();
    println!("branch: {:?}", &branch);
    println!("commit: {:?}", &commit);
    println!("org_name: {:?}", &org_name);
    println!("repo_name: {:?}", &repo_name);

    let branch_dir = &args.artifacts_dir.join(branch);
    let work_dir = &branch_dir.join("work");

    if !branch_dir.exists() {
        println!("Creating branch_dir {:?}", &branch_dir);
        fs::create_dir(&branch_dir)
            .expect(format!("Could not create branch_dir {:?}", &branch_dir).as_str());
    }

    if let Ok(keypair) = generate_keypair(args.provider_token).await {
        println!(
            "Generated keypair {:?}",
            &keypair.private_key_to_pem().unwrap()
        );
        let priv_key = keypair.private_key_to_pem().unwrap();
        let priv_key = priv_key_from_pem(&priv_key).unwrap();

        let public_key_pem = keypair.public_key_to_pem().unwrap();
        let public_key_path = branch_dir.join("cosign.pub");
        fs::write(public_key_path, public_key_pem).expect("Could not write public key");

        // Generate layout
        let signed_mb = create_layout(&org_name, &repo_name, &priv_key, args.valid_days).unwrap();

        let verified_mb = signed_mb.verify(1, [priv_key.public()]);
        if verified_mb.is_err() {
            eprintln!("Could not verify metadata: {:?}", verified_mb.err());
            std::process::exit(1);
        }
        let filename = format!("{}/{}-layout.json", &branch_dir.display(), &repo_name);
        let s = serde_json::to_string_pretty(&signed_mb).unwrap();
        fs::write(filename, s).unwrap();
        println!(
            "Generate {}/{}-layout.json",
            &branch_dir.display(),
            &repo_name
        );

        // Create steps
        if !work_dir.exists() {
            println!("Creating work dir {:?}", &work_dir);
            fs::create_dir(&work_dir)
                .expect(format!("Could not create work directory {:?}", &work_dir).as_str());
        }

        println!("key_id: {:?}", priv_key.key_id().prefix());

        // Generate clone-project step
        let clone_project = runlib::in_toto_run(
            "clone-project",                   // name
            Some(&work_dir.to_str().unwrap()), // workdir
            &[
                // materials
                "Cargo.toml",
                "Cargo.lock",
                "README.md",
                "src",
            ],
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
            Some(&["sha256"]),
            None,
        )
        .unwrap();

        let json = serde_json::to_value(&clone_project).unwrap();

        let filename = format!("{}.{}.link", "clone-project", priv_key.key_id().prefix());
        let path = &branch_dir.join(&filename);
        let s = serde_json::to_string_pretty(&json).unwrap();
        fs::write(&path, s).unwrap();
        println!("Generated {}", path.display());

        println!("Verify clone-project step...");
        clone_project.verify(1, [priv_key.public()]).unwrap();

        // Generate run-tests step
        let run_tests = runlib::in_toto_run(
            "run-tests",                      // name
            Some(work_dir.to_str().unwrap()), // workdir
            &[""],                            // materials
            &[""],
            &["cargo", "test"],
            Some(&priv_key),
            Some(&["sha256"]),
            Some(&["source-distributed"]),
        )
        .unwrap();

        let json = serde_json::to_value(&run_tests).unwrap();
        let filename = format!("{}.{}.link", "run-tests", priv_key.key_id().prefix());
        let path = branch_dir.join(&filename);
        let s = serde_json::to_string_pretty(&json).unwrap();
        fs::write(&path, s).unwrap();
        println!("Generated {}", path.display());
        println!("Verify run-steps...");
        run_tests.verify(1, [priv_key.public()]).unwrap();

        fs::remove_dir_all(&work_dir).unwrap();
    } else {
        eprintln!("Could not generate keypair");
    }
}
