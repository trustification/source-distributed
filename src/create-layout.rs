use chrono::{offset::Local, DateTime, Days, Utc};
use clap::Parser;
use in_toto::crypto::PrivateKey;
use in_toto::models::inspection::Inspection;
use in_toto::models::rule::{Artifact, ArtifactRule};
use in_toto::models::step::{Command, Step};
use in_toto::models::VirtualTargetPath;
use in_toto::models::{LayoutMetadataBuilder, Metablock, MetablockBuilder};
use source_distributed::private_key_from_file;
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
        help = "The github organisation that the project/repository belongs to"
    )]
    org_name: String,

    #[arg(short, long, help = "The github repository/project")]
    repo_name: String,

    #[arg(long, help = "The private key to be used to sign the layout")]
    private_key: PathBuf,

    #[arg(
        long,
        help = "The directory to store the artifacts in.",
        default_value = "artifacts_work"
    )]
    artifacts_dir: String,

    #[arg(
        long,
        help = "The number of days that the layout should be valid",
        default_value = "365"
    )]
    valid_days: u64,
}

fn create_layout(
    org_name: &String,
    repo_name: &String,
    priv_key: &PrivateKey,
    valid_days: u64,
) -> in_toto::Result<Metablock> {
    println!("private keyid: {:?}", priv_key.key_id());
    println!("public keyid: {:?}", priv_key.public().key_id());
    let expires: DateTime<Utc> = DateTime::from(
        Local::now()
            .checked_add_days(Days::new(valid_days))
            .unwrap(),
    );
    let metadata = LayoutMetadataBuilder::new()
        .expires(expires)
        .readme(format!("in-toto layout for {}/{}.", org_name, repo_name).to_string())
        .add_key(priv_key.public().to_owned())
        .add_step(
            Step::new("clone-project")
                .threshold(1)
                .add_expected_product(ArtifactRule::Create(repo_name.as_str().into()))
                .add_expected_product(ArtifactRule::Allow(
                    VirtualTargetPath::new(format!("{}/*", repo_name)).unwrap(),
                ))
                .add_expected_product(ArtifactRule::Allow(
                    VirtualTargetPath::new(format!("{}-layout.json", repo_name)).unwrap(),
                ))
                .add_key(priv_key.key_id().to_owned())
                .expected_command(
                    format!("git clone git@github.com:{}/{}.git", org_name, repo_name).into(),
                ),
        )
        .add_step(
            Step::new("run-tests")
                .threshold(1)
                .add_expected_material(ArtifactRule::Match {
                    pattern: format!("{}/*", repo_name).as_str().into(),
                    in_src: None,
                    with: Artifact::Products,
                    in_dst: None,
                    from: "clone-project".into(),
                })
                .add_expected_material(ArtifactRule::Allow("Cargo.toml".into()))
                .add_expected_material(ArtifactRule::Disallow("*".into()))
                .add_expected_product(ArtifactRule::Allow("Cargo.lock".into()))
                .add_expected_product(ArtifactRule::Allow("cosign.key.json".into()))
                .add_expected_product(ArtifactRule::Allow("cosign.key.pub.json".into()))
                .add_expected_product(ArtifactRule::Disallow("*".into()))
                .add_key(priv_key.key_id().to_owned())
                .expected_command(
                    format!("cargo test --manifest-path={}/Cargo.toml", repo_name).into(),
                ),
        )
        .add_inspect(
            Inspection::new("cargo-fetch")
                .add_expected_material(ArtifactRule::Match {
                    pattern: format!("{}/*", repo_name).as_str().into(),
                    in_src: None,
                    with: Artifact::Products,
                    in_dst: None,
                    from: "clone-project".into(),
                })
                .add_expected_material(ArtifactRule::Allow(
                    format!("{}/target", repo_name).as_str().into(),
                ))
                .add_expected_material(ArtifactRule::Allow("cosign.key.pub.json".into()))
                .add_expected_material(ArtifactRule::Allow(
                    format!("{}-layout.json", repo_name).as_str().into(),
                ))
                .add_expected_material(ArtifactRule::Disallow("*".into()))
                .add_expected_product(ArtifactRule::Match {
                    pattern: format!("{}/Cargo.toml", repo_name).as_str().into(),
                    in_src: None,
                    with: Artifact::Products,
                    from: "clone-project".into(),
                    in_dst: None,
                })
                .add_expected_product(ArtifactRule::Match {
                    pattern: "*".into(),
                    in_src: None,
                    with: Artifact::Products,
                    from: "clone-project".into(),
                    in_dst: None,
                })
                .add_expected_product(ArtifactRule::Allow(
                    format!("{}/target", repo_name).as_str().into(),
                ))
                .add_expected_product(ArtifactRule::Allow("cosign.key.pub.json".into()))
                .add_expected_product(ArtifactRule::Allow(
                    format!("{}-layout.json", repo_name).as_str().into(),
                ))
                .run(Command::from(format!(
                    "git clone git@github.com:{}/{}.git",
                    org_name, repo_name
                ))),
        )
        .build()?;

    let signed_metablock_builder =
        MetablockBuilder::from_metadata(Box::new(metadata)).sign(&[&priv_key])?;
    Ok(signed_metablock_builder.build())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let org_name = args.org_name;
    let repo_name = args.repo_name;
    println!("Generate in-toto layout for {}/{}", org_name, repo_name);

    let priv_key = private_key_from_file(&args.private_key);

    let signed_mb = create_layout(&org_name, &repo_name, &priv_key, args.valid_days).unwrap();
    //println!("{:?}", metablock.signatures());
    let verified_mb = signed_mb.verify(1, [priv_key.public()]);
    if verified_mb.is_err() {
        eprintln!("Could not verify metadata: {:?}", verified_mb.err());
        std::process::exit(1);
    }
    let filename = format!("{}/{}-layout.json", args.artifacts_dir, repo_name);
    let s = serde_json::to_string_pretty(&signed_mb).unwrap();
    fs::write(filename, s).unwrap();
    println!(
        "Generate {}/{}-layout.json",
        &args.artifacts_dir, &repo_name
    );
}

#[cfg(test)]
mod test {
    use source_distributed::{create_layout, priv_key_from_pem};
    #[test]
    fn test_create_layout() {
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
        let metablock = create_layout(
            &org_name.to_string(),
            &repo_name.to_string(),
            &priv_key,
            365,
        )
        .unwrap();
        let v = metablock.verify(1, [priv_key.public()]);
        assert!(v.is_ok());
    }
}
