use clap::Parser;
use source_distributed::git::CargoGit;

#[derive(Parser, Debug)]
#[command(author,
    version,
    long_about = None)]
/// prints the hash of a github url
struct Args {
    #[arg(short, long, help = "The github url to hash")]
    url: String,
}

fn main() {
    let args = Args::parse();
    let url = args.url;
    println!("{}: {}", &url, CargoGit::hash_url(&url));
}
