use anyhow::{anyhow, Result};
use in_toto::crypto::{PrivateKey, SignatureScheme};
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
