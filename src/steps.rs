use in_toto::crypto::PrivateKey;
use in_toto::models::Metablock;
use in_toto::runlib;
use std::fs;
use std::path::PathBuf;

pub fn clone_project(
    org_name: &str,
    repo_name: &str,
    priv_key: &PrivateKey,
    work_dir: PathBuf,
) -> anyhow::Result<Metablock> {
    // Generate clone-project step
    let clone_project = runlib::in_toto_run(
        "clone-project",                   // name
        Some(&work_dir.to_str().unwrap()), // workdir
        &[
            // materials
            "Cargo.toml",
            "Cargo.lock",
            "README.md",
            "src",
        ],
        &[
            // products
            "Cargo.toml",
            "Cargo.lock",
            "README.md",
            "src",
        ],
        &[
            "git",
            "clone",
            format!("git@github.com:{}/{}.git", org_name, repo_name).as_str(),
        ],
        Some(&priv_key),
        Some(&["sha256"]),
        None,
    )
    .unwrap();

    println!("Verify clone-project step...");
    // TODO: Add a flag to optionally verify the step, but I've found this
    // useful during development as there have been issues with signatures,
    // for example https://github.com/in-toto/in-toto-rs/pull/48.
    clone_project.verify(1, [priv_key.public()]).unwrap();
    Ok(clone_project)
}

pub fn run_tests(priv_key: &PrivateKey, work_dir: &PathBuf) -> anyhow::Result<Metablock> {
    // Generate run-tests step
    let run_tests = runlib::in_toto_run(
        "run-tests",                      // name
        Some(work_dir.to_str().unwrap()), // workdir
        &[""],                            // materials
        &[""],
        &["cargo", "test"],
        Some(&priv_key),
        Some(&["sha256"]),
        Some(&["source-distributed"]),
    )
    .unwrap();
    println!("Verify run_tests step...");
    run_tests.verify(1, [priv_key.public()]).unwrap();
    Ok(run_tests)
}

pub fn write_layout_to_file(
    layout: &Metablock,
    repo_name: &str,
    dir: &PathBuf,
) -> anyhow::Result<PathBuf> {
    let filename = format!("{}/{}-layout.json", &dir.display(), &repo_name);
    let content = serde_json::to_string_pretty(&layout).unwrap();
    fs::write(&filename, content).unwrap();
    println!("Generate {}/{}-layout.json", &dir.display(), &repo_name);
    Ok(PathBuf::from(&filename))
}

pub fn write_step_to_file(
    step: &Metablock,
    step_name: &str,
    priv_key: &PrivateKey,
    dir: &PathBuf,
) -> anyhow::Result<PathBuf> {
    let json = serde_json::to_value(&step).unwrap();
    let filename = format!("{}.{}.link", step_name, priv_key.key_id().prefix());
    let path = &dir.join(&filename);
    let s = serde_json::to_string_pretty(&json).unwrap();
    fs::write(&path, s).unwrap();
    println!("Generated {}", path.display());
    Ok(path.to_path_buf())
}
