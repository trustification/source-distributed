use anyhow::{anyhow, Result};
use chrono::{offset::Local, DateTime, Days, Utc};
use in_toto::crypto::{PrivateKey, SignatureScheme};
use in_toto::models::inspection::Inspection;
use in_toto::models::rule::{Artifact, ArtifactRule};
use in_toto::models::step::{Command, Step};
use in_toto::models::VirtualTargetPath;
use in_toto::models::{LayoutMetadataBuilder, Metablock, MetablockBuilder};
use openidconnect::core::CoreIdToken;
use sigstore::crypto::signing_key::SigStoreKeyPair;
use sigstore::crypto::SigningScheme;
use sigstore::fulcio::oauth::OauthTokenProvider;
use sigstore::fulcio::{FulcioClient, TokenProvider, FULCIO_ROOT};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;
use x509_parser::pem::parse_x509_pem;

pub mod steps;

/// Parses the passed in string which is expected to be a private ec key in
/// pem format, or in securesystemslib json format.
pub fn priv_key_from_pem(s: &str) -> in_toto::Result<PrivateKey> {
    let json_format: serde_json::Result<serde_json::Value> = serde_json::from_str(s);
    if let Ok(_) = json_format {
        let priv_key = PrivateKey::from_securesystemslib_ecdsa(s).unwrap();
        return Ok(priv_key);
    } else {
        let (_, der) = parse_x509_pem(s.as_bytes()).unwrap();
        let priv_key =
            PrivateKey::from_pkcs8(&der.contents, SignatureScheme::EcdsaP256Sha256).unwrap();
        return Ok(priv_key);
    }
}

pub fn private_key_from_file(path: &PathBuf) -> PrivateKey {
    let private_key_pem = fs::read_to_string(path).unwrap();
    priv_key_from_pem(&private_key_pem).unwrap()
}

pub async fn generate_keypair(token: Option<String>) -> Result<SigStoreKeyPair> {
    let token_provider = match token {
        Some(token) => {
            let id_token: CoreIdToken = CoreIdToken::from_str(&token).unwrap();
            TokenProvider::Static((id_token, "keygen".to_string()))
        }
        _ => TokenProvider::Oauth(OauthTokenProvider::default()),
    };

    let fulcio = FulcioClient::new(Url::parse(FULCIO_ROOT).unwrap(), token_provider);

    if let Ok((signer, _cert)) = fulcio
        .request_cert(SigningScheme::ECDSA_P256_SHA256_ASN1)
        .await
    {
        let keypair = signer.to_sigstore_keypair().unwrap();
        return Ok(keypair);
    }
    Err(anyhow!("Could not generate keypair"))
}

pub fn create_layout(
    org_name: &str,
    repo_name: &str,
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
        eprintln!("Could not verify metadata: {:?}", verified.err());
        std::process::exit(1);
    }
    Ok(signed)
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
