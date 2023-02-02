use anyhow::{anyhow, Result};
use in_toto::crypto::{PrivateKey, SignatureScheme};
use jsonwebtoken::jwk::AlgorithmParameters;
use jsonwebtoken::{decode, decode_header, jwk, DecodingKey, Validation};
use openidconnect::core::CoreIdToken;
use sigstore::crypto::signing_key::SigStoreKeyPair;
use sigstore::crypto::SigningScheme;
use sigstore::fulcio::oauth::OauthTokenProvider;
use sigstore::fulcio::{FulcioClient, TokenProvider, FULCIO_ROOT};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;
use x509_parser::pem::parse_x509_pem;

/// Parses the passed in string which is expected to be a private ec key in
/// pem format, or in securesystemslib json format.
pub fn priv_key_from_pem(s: &str) -> in_toto::Result<PrivateKey> {
    let (_, der) = parse_x509_pem(s.as_bytes()).unwrap();
    let priv_key = PrivateKey::from_pkcs8(&der.contents, SignatureScheme::EcdsaP256Sha256).unwrap();
    Ok(priv_key)
}

pub fn private_key_from_file(path: &PathBuf) -> PrivateKey {
    let private_key_pem = fs::read_to_string(path).unwrap();
    priv_key_from_pem(&private_key_pem).unwrap()
}

fn extract_subject(token: &str, jwks_path: &str) -> String {
    let header = decode_header(&token).unwrap();
    let jwt_base64 = fs::read_to_string(jwks_path).unwrap();
    let jwks: jwk::JwkSet = serde_json::from_str(&jwt_base64).unwrap();
    let keyid = jwks.find(&header.kid.as_ref().unwrap());
    if keyid.is_none() {
        eprintln!(
            "Could not find a matching key for keyid {:?}",
            &header.kid.as_ref().unwrap()
        );
    }
    let keyid = keyid.unwrap();
    let subject = match &keyid.algorithm {
        AlgorithmParameters::RSA(rsa) => {
            let decoding_key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e).unwrap();
            let mut validation = Validation::new(keyid.common.algorithm.unwrap());
            validation.validate_exp = false;
            validation.validate_nbf = false;
            let decoded_token =
                decode::<HashMap<String, serde_json::Value>>(&token, &decoding_key, &validation)
                    .unwrap();
            println!("{:?}", &decoded_token);
            decoded_token.claims.get("sub").unwrap().clone()
        }
        _ => unreachable!("this should be a RSA"),
    };
    subject.to_string()
}

pub async fn generate_keypair(token: Option<String>) -> Result<SigStoreKeyPair> {
    let token_provider = match token {
        Some(token) => {
            // Just writing the token to disk which can be useful for
            // debugging a CI workflow
            fs::write("token", &token).unwrap();

            // See ./notes.md##coreidtoken for details about the usage of the
            // subject and JSON Web Key Sets (JWKS))
            let subject = extract_subject(&token, "jwks");
            println!("subject: {}", &subject);
            println!(
                "subject: {}",
                "repo:trustification/source-distributed:ref:refs/heads/main".to_string()
            );
            let id_token: CoreIdToken = CoreIdToken::from_str(&token).unwrap();
            TokenProvider::Static((id_token, subject.to_string()))
            /*
            TokenProvider::Static((
                id_token,
                "repo:trustification/source-distributed:ref:refs/heads/main".to_string(),
            ))
            */
        }
        _ => TokenProvider::Oauth(OauthTokenProvider::default()),
    };

    let fulcio = FulcioClient::new(Url::parse(FULCIO_ROOT).unwrap(), token_provider);

    if let Ok((signer, _cert)) = fulcio
        .request_cert(SigningScheme::ECDSA_P256_SHA256_ASN1)
        .await
    {
        println!("cert: {}", &_cert);
        let keypair = signer.to_sigstore_keypair().unwrap();
        return Ok(keypair);
    }
    Err(anyhow!("Could not generate keypair"))
}
