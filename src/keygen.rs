use openidconnect::core::CoreIdToken;
use sigstore::crypto::SigningScheme;
use sigstore::fulcio::oauth::OauthTokenProvider;
use sigstore::fulcio::{FulcioClient, TokenProvider, FULCIO_ROOT};
use std::env;
use std::fs;
use std::str::FromStr;
use url::Url;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let token_provider = match args.len() {
        2 => {
            let id_token: CoreIdToken = CoreIdToken::from_str(&args[1]).unwrap();
            TokenProvider::Static((id_token, "keygen".to_string()))
        }
        _ => TokenProvider::Oauth(OauthTokenProvider::default()),
    };
    let fulcio = FulcioClient::new(Url::parse(FULCIO_ROOT).unwrap(), token_provider);

    if let Ok((signer, cert)) = fulcio
        .request_cert(SigningScheme::ECDSA_P256_SHA256_ASN1)
        .await
    {
        let keypair = signer.to_sigstore_keypair().unwrap();
        let private_key_pem = keypair.private_key_to_pem().unwrap();
        fs::write("cosign.key", private_key_pem).expect("Could not write private key");
        let public_key_pem = keypair.public_key_to_pem().unwrap();
        fs::write("cosign.pub", public_key_pem).expect("Could not write public key");
        fs::write("cosign.crt", cert).expect("Could not write certificate key");
    } else {
        println!("was not able to create keypair");
    }
}
