## Source distributed project
This project contains some example code developed while investigating how
source distrubted Rust projects might be able to use Sigstore, and in-toto, to
sign and verify source distributed project artifacts.

For more more background information please see [notes.md](./notes.md) which
documents some of issues we ran into while doing our investiation.

### Signing/Generating
To sign a project, the following command can be used:
```console
$ cargo r --bin cargo-in-toto-sign
```
This will use Sigstore's ephemeral keys (keyless) feature to generate a keypair
that will then be used to sign the in-toto artifacts. The artifacts will be
stored in [sscs/in-toto/artifacts/<branch>](./sscs/in-toto/artifacts) depending
on the current branch.

### Verifying
To verify a project we need to specify which dependency from Cargo.toml that
we want to verify:
```console
$ cargo r --bin cargo-in-toto-verify -- -d source-distributed
```
The above will verify that the branch specified for this dependency, in this
case `main`.

The following option can be used to check a directory that is outside of
`~/.cargo/git`:
```console
$ cargo r --bin cargo-in-toto-verify -- -d source-distributed -a sscs/in-toto/artifacts/main -p $PWD

```

### Logging
Currently logging is done using the log crate and env_logger is the
implementation used. This can be configured using:
```console
$ env RUST_LOG=cargo_in_toto_signn=debug cargo r --bin cargo-in-toto-sign
```

### Running the CI workflow locally
The same workflow that the github action runs can be run locally, in which case
there the OIDC flow will open a browser to choose the OICD Provider to use:
```console
$ cd sscs/in-toto
$ ./workflow
```
The output of the command will then be available in
[artifacts](./sscs/in-toto/artifacts).
