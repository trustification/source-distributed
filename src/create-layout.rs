use chrono::{offset::Local, DateTime, Days, Utc};
use clap::Parser;
//use in_toto::crypto::PublicKey;
use in_toto::models::{LayoutMetadata, LayoutMetadataBuilder};
use std::fs;
/*
use url::Url;
*/

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
    private_key: String,

    #[arg(
        long,
        help = "The public key belonging to the private key which will be stored in the metadata"
    )]
    public_key: String,
}

fn create_layout(
    org_name: &String,
    repo_name: &String,
    private_key_pem: &String,
    public_key_pem: &String,
) -> in_toto::Result<LayoutMetadata> {
    let expires: DateTime<Utc> =
        DateTime::from(Local::now().checked_add_days(Days::new(365)).unwrap());
    LayoutMetadataBuilder::new()
        .expires(expires)
        .readme(format!("in-toto layout for {}/{}.", org_name, repo_name).to_string())
        .build()
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let org_name = args.org_name;
    let repo_name = args.repo_name;
    println!("Generate in-toto layout for {}/{}", org_name, repo_name);

    let private_key_pem = fs::read_to_string(&args.private_key).unwrap();
    println!("{:?}", &private_key_pem);

    let public_key_pem = fs::read_to_string(&args.public_key).unwrap();
    println!("{:?}", &public_key_pem);

    let metadata = create_layout(&org_name, &repo_name, &private_key_pem, &public_key_pem);
    println!("{:?}", metadata);
}

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
    let public_key_pem = r#"
-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE2sur8bUks+UctwaYxb2/icKHzzrL
Vbe2VCYOco36pFZBwL2vy7+67YutPDayUejWbBw+mlI7McEB1DIyfKyIrQ==
-----END PUBLIC KEY-----
"#;
    let metadata = create_layout(
        &org_name.to_string(),
        &repo_name.to_string(),
        &private_key_pem.to_string(),
        &public_key_pem.to_string(),
    );
    println!("{:?}", metadata);
}
