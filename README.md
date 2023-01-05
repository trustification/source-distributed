## Source distributed project
This project contains some example code developed while investigating how
source distrubted Rust projects might be able to use Sigstore, and in-toto, to
sign and verify source distributed project artifacts.

For more more background information please see [notes.md](./notes.md) which
documents some of issues we ran into while doing our investiation.

### Signing/Generating
To sign a project, the following command can be used:
```console
$ cargo r --bin cargo-in-toto-gen
```
This will use Sigstore's ephemeral keys (keyless) feature to generate a keypair
that will then be used to sign the in-toto artifacts.

### Verifying
To verify a project we need to specify which dependency from Cargo.toml that
we want to verify:
```console
$ cargo r --bin cargo-verify -- -d source-distributed
```

The following option can be used to check a directory that is outside of
`~/.cargo/git`:
```console
$ cargo r --bin cargo-verify -- -d source-distributed -a sscs/in-toto/artifacts/main -p $PWD

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
