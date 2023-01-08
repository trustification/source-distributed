use chrono::{offset::Local, DateTime, Days, Utc};
use in_toto::crypto::PrivateKey;
use in_toto::models::inspection::Inspection;
use in_toto::models::rule::{Artifact, ArtifactRule};
use in_toto::models::step::{Command, Step};
use in_toto::models::VirtualTargetPath;
use in_toto::models::{LayoutMetadataBuilder, Metablock, MetablockBuilder};
use log::{debug, error};

pub fn create_layout(
    org_name: &str,
    repo_name: &str,
    priv_key: &PrivateKey,
    valid_days: u64,
) -> in_toto::Result<Metablock> {
    debug!("private keyid: {:?}", priv_key.key_id());
    debug!("public keyid: {:?}", priv_key.public().key_id());
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
                .add_expected_product(ArtifactRule::Create(repo_name.into()))
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
                //.add_expected_material(ArtifactRule::Disallow("*".into()))
                .add_expected_product(ArtifactRule::Allow("Cargo.lock".into()))
                .add_expected_product(ArtifactRule::Allow("cosign.key.json".into()))
                .add_expected_product(ArtifactRule::Allow("cosign.key.pub.json".into()))
                //.add_expected_product(ArtifactRule::Disallow("*".into()))
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
                .add_expected_material(ArtifactRule::Allow("cosign.pub".into()))
                .add_expected_material(ArtifactRule::Allow(
                    format!("{}-layout.json", repo_name).as_str().into(),
                ))
                //.add_expected_material(ArtifactRule::Disallow("*".into()))
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
    let signed = signed_metablock_builder.build();
    let verified = signed.verify(1, [priv_key.public()]);
    if verified.is_err() {
        error!("Could not verify metadata: {:?}", verified.err());
        std::process::exit(1);
    }
    Ok(signed)
}
