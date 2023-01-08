use anyhow::anyhow;

pub mod crypto;
pub mod layout;
pub mod steps;

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
