use in_toto::crypto::{PrivateKey, SignatureScheme};
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
