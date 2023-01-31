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

pub async fn generate_keypair(token: Option<String>) -> Result<SigStoreKeyPair> {
    let token_provider = match token {
        Some(token) => {
            let header = decode_header(&token).unwrap();
            println!("{:?}", &header);
            println!("typ: {:?}", &header.typ);
            println!("alg: {:?}", &header.alg);
            println!("kid: {:?}", &header.kid.as_ref().unwrap());
            println!("x5t: {:?}", &header.x5t.unwrap());

            let jwt_base64 = fs::read_to_string("jwks").unwrap();
            let jwks: jwk::JwkSet = serde_json::from_str(&jwt_base64).unwrap();
            if let Some(j) = jwks.find(&header.kid.unwrap()) {
                match &j.algorithm {
                    AlgorithmParameters::RSA(rsa) => {
                        let decoding_key =
                            DecodingKey::from_rsa_components(&rsa.n, &rsa.e).unwrap();
                        let mut validation = Validation::new(j.common.algorithm.unwrap());
                        validation.validate_exp = false;
                        validation.validate_nbf = false;
                        let decoded_token = decode::<HashMap<String, serde_json::Value>>(
                            &token,
                            &decoding_key,
                            &validation,
                        )
                        .unwrap();
                        println!("claims: {:?}", decoded_token.claims);
                        println!("sub: {:?}", decoded_token.claims.get("sub").unwrap());
                        println!("aud: {:?}", decoded_token.claims.get("aud").unwrap());
                        println!("iss: {:?}", decoded_token.claims.get("iss").unwrap());
                        println!(
                            "job_workflow_ref: {:?}",
                            decoded_token.claims.get("job_workflow_ref").unwrap()
                        );
                    }
                    _ => unreachable!("this should be a RSA"),
                }
            } else {
                eprintln!("No matching JWK found for the given kid");
            }

            fs::write("token", &token).unwrap();
            let id_token: CoreIdToken = CoreIdToken::from_str(&token).unwrap();
            TokenProvider::Static((
                id_token,
                "repo:trustification/source-distributed:ref:refs/heads/main".to_string(),
            ))
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
