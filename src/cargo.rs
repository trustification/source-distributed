/// This function filters out the passed in binary name which is expected to
/// be the name of a cargo extension but without the 'cargo' prefix.
///
/// For example, the cargo-in-toto-verify application when installed using
/// `cargo install` and then called using `cargo in-toto-verify`, the args
/// passed to the program will be:
/// ```text
///   ["target/debug/cargo-in-toto-verify", "in-toto-verify", "--help"]
/// ```
///
/// Trying run this program using `cargo in-toto-verify` will produce the
/// following error:
/// ```console
/// $ cargo in-toto-verify --help
/// error: Found argument 'in-toto-verify' which wasn't expected, or isn't valid in this context
/// ```
///
/// After filtering by this function this will become:
/// ```text
///   ["target/debug/cargo-in-toto-verify", "--help"]
/// ```
/// And this will allow program to work as expected.
pub fn filter_args(cargo_bin_name: &str) -> Vec<String> {
    let mut args = std::env::args();
    let args = args.by_ref();
    let filtered: Vec<String> = args.filter(|a| !a.starts_with(cargo_bin_name)).collect();
    filtered
}
