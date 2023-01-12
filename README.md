## Source distributed project
This project contains some example code developed while investigating how
source distrubted Rust projects might be able to use Sigstore, and in-toto, to
sign and verify source distributed project artifacts.

### Motivation
The motivation for doing this is the issue that many projects depend on open
source projects as dependencies. If there is a vulnerability in one of these
dependencies that means that the projects depending on them are also vulnerabe.
A [problem](https://www.softwaremaxims.com/blog/not-a-supplier) with this open
source software supply chain is that many of these open source projects are
maintained by people in there spare time, not many are actually paid to work on
them. But these project need to be secured at some point if we want projects
that depend on them to also be secure, and consumers need to be able to verify
them to have a secure product themselves.

How can get these maintainers secure their projects?  
By making it as simple as possible to do. 

### Goal
The goals is to provide a tools that can be used to add Secure Supply Chain
Security (SSCS) artifacts to a project. The in-toto steps that are generated are
a clone of the project, and running of the projects tests. The in-toto layout
generated will allow for these steps to be verified.

For more more background information please see [notes.md](./notes.md) which
documents some of issues we ran into while doing our investigations.

### Suggestion/Solution
We are suggesting that `in-toto` be used to generate artifacts for the projects,
initially this is very simple as creating steps for cloning a project and
running the tests in it. These steps will be signed and can later be
inspected/verified by consumers.

Signing means that keys need to be used and here we are using `Sigstore` and its
ephemeral key solution which simplifies the key management in a similar manner
to how `Let's Encrypt` simplified certificate managment.

### Installing
The binaries can be installed using the following command:
```console
$ cargo install --path .
```
And then be run using:
```console
$ cargo in-toto-verify --help
```

### Signing/Generating
This tool would be run by a maintainer to generate securtiy artifacts for their
project:
```console
$ cargo in-toto-sign
```
This will use Sigstore's ephemeral keys (keyless) feature to generate a keypair
that will then be used to sign the in-toto artifacts. The artifacts will be
stored in [sscs/in-toto/artifacts/branch](./sscs/in-toto/artifacts) depending
on the current branch. These files should be checked in and they will later be
used when verifying.

### Verifying
This tool is intended to be run by a consumer/user.

To verify a project we need to specify which dependency from Cargo.toml that
is to be verified:
```console
$ cargo in-toto-verify -- -d source-distributed
```
The above will verify that the branch specified for this dependency, in this
case `main`.

The following option can be used to check a directory that is outside of
`~/.cargo/git`:
```console
$ cargo in-toto-verify -- -d source-distributed -a sscs/in-toto/artifacts/main -p $PWD
```

To verify the current project instead of a dependency the `--current-project`
option can be specified:
```console
$ cargo in-toto-verify -- -c
```
This could be useful for project maintainers to verify the artifacts in the same
project as generated by in-toto-verify.

### Logging
Currently logging is done using the log crate and env_logger is the
implementation used. This can be configured using:
```console
$ env RUST_LOG=cargo_in_toto_sign=debug cargo r --bin cargo-in-toto-sign
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
