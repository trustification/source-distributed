use cargo::core::source::SourceId;
use cargo::util::hex::short_hash;
use cargo::util::Config;

fn main() {
    let config = Config::default().unwrap();
    let registry_id = SourceId::crates_io(&config).unwrap();
    let hash = format!("{}", short_hash(&registry_id));
    println!("{}: {}", registry_id.display_registry_name(), hash);
}
