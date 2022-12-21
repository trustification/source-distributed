use clap::Parser;
use in_toto::runlib;
use source_distributed::priv_key_from_pem;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author,
    version,
    long_about = None)]
/// create-steps generates ...
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
    private_key: String,

    #[arg(short, long, help = "The name of the step")]
    step_name: String,

    #[arg(
        long,
        help = "The directory to store the artifacts in.",
        default_value = "sscs/in-toto"
    )]
    artifacts_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let org_name = args.org_name;
    let repo_name = args.repo_name;
    println!("Generate in-toto step for {}/{}", org_name, repo_name);

    let private_key_pem = fs::read_to_string(&args.private_key).unwrap();
    let priv_key = priv_key_from_pem(&private_key_pem).unwrap();
    println!("key_id: {:?}", priv_key.key_id().prefix());

    let link = runlib::in_toto_run(
        &args.step_name, // name
        Some("work"),    // workdir
        &[""],           // materials
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

#[test]
fn test_create_step() {
    let org_name = "someorg";
    let repo_name = "somerepo";
    let private_key_pem = r#"
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQga+rUgQvB60AIJZL1
YBLG6iIMRoTDjAZ6IcRYK2XtuGuhRANCAATay6vxtSSz5Ry3BpjFvb+JwofPOstV
t7ZUJg5yjfqkVkHAva/Lv7rti608NrJR6NZsHD6aUjsxwQHUMjJ8rIit
-----END PRIVATE KEY-----
"#;
    let priv_key = priv_key_from_pem(&private_key_pem).unwrap();
    assert!(true);
}
