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

### `cargo sign` vs `in-toto-sign`
But what about the efforts that are underway to enable signing of a crate upon
publishing it, and then being able to verify the crate by a consumer, is this
not duplicating that effort?  
With the signing of crates, that is performed upon publishing the crate to
crates.io and it is the `.crate` tar file that is signed, but there is still a
possiblity that the sources published do not match the sources in git. in-toto
can add this extra level of verification making sure that the sources actually
match.

For example, lets create a new cargo crate and run the `cargo package` command:
```console
$ cargo new cargo-package && cd cargo-package
$ cargo package --allow-dirty
warning: manifest has no description, license, license-file, documentation, homepage or repository.
See https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata for more info.
   Packaging cargo-package v0.1.0 (/home/danielbevenius/work/rust/learning-rust/cargo-package)
   Verifying cargo-package v0.1.0 (/home/danielbevenius/work/rust/learning-rust/cargo-package)
   Compiling cargo-package v0.1.0 (/home/danielbevenius/work/rust/learning-rust/cargo-package/target/package/cargo-package-0.1.0)
    Finished dev [unoptimized + debuginfo] target(s) in 0.43s
```
This will create a .crate file:
```console
$ file target/package/cargo-package-0.1.0.crate 
target/package/cargo-package-0.1.0.crate: gzip compressed data, was "cargo-package-0.1.0.crate", max compression, original size modulo 2^32 5632
```
We can list the contents of this using:
```console
$ tar tvf target/package/cargo-package-0.1.0.crate 
-rw-r--r-- 0/0             157 1970-01-01 01:00 cargo-package-0.1.0/Cargo.lock
-rw-r--r-- 0/0             551 1970-01-01 01:00 cargo-package-0.1.0/Cargo.toml
-rw-r--r-- 0/0             182 2006-07-24 03:21 cargo-package-0.1.0/Cargo.toml.orig
-rw-r--r-- 0/0              45 2006-07-24 03:21 cargo-package-0.1.0/src/main.rs
```
Now, we can sign the crate using Sigstore (which is similar to what `cargo sign`
will do later I think):
```console
$ COSIGN_EXPERIMENTAL=1 cosign sign-blob --bundle=artifact.bundle target/package/cargo-package-0.1.0.crate 
Using payload from: target/package/cargo-package-0.1.0.crate
Generating ephemeral keys...
Retrieving signed certificate...

        Note that there may be personally identifiable information associated with this signed artifact.
        This may include the email address associated with the account with which you authenticate.
        This information will be used for signing this artifact and will be stored in public transparency logs and cannot be removed later.
        By typing 'y', you attest that you grant (or have permission to grant) and agree to have this information stored permanently in transparency logs.

Are you sure you want to continue? (y/[N]): y
Your browser will now be opened to:
https://oauth2.sigstore.dev/auth/auth?access_type=online&client_id=sigstore&code_challenge=nJDxvWxIEpmabcifrNS3R-RJmqsFjoSVaZHv7UnHE8I&code_challenge_method=S256&nonce=2KOpTiqUq7K2RlSefOmbC0oYMap&redirect_uri=http%3A%2F%2Flocalhost%3A41853%2Fauth%2Fcallback&response_type=code&scope=openid+email&state=2KOpTjJ8js64nxydUHDgitnA7Pt
Successfully verified SCT...
using ephemeral certificate:
-----BEGIN CERTIFICATE-----
MIICpzCCAi6gAwIBAgIUPFVeebq5GpWjWa5AyAlwTqDDmYowCgYIKoZIzj0EAwMw
NzEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MR4wHAYDVQQDExVzaWdzdG9yZS1pbnRl
cm1lZGlhdGUwHhcNMjMwMTE2MDc1MTM3WhcNMjMwMTE2MDgwMTM3WjAAMFkwEwYH
KoZIzj0CAQYIKoZIzj0DAQcDQgAE2a8bkOVFPkJK/7MyjKkeOgh6f8g3/X9Hs2oL
7djTmTQpNAiSeVqkgolQaodIrebaK74kmEKgXnGr5oImAVMIMaOCAU0wggFJMA4G
A1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDAzAdBgNVHQ4EFgQU0fpg
WH23kDbBwS1a+rlDjCPOvLAwHwYDVR0jBBgwFoAU39Ppz1YkEZb5qNjpKFWixi4Y
ZD8wJwYDVR0RAQH/BB0wG4EZZGFuaWVsLmJldmVuaXVzQGdtYWlsLmNvbTAsBgor
BgEEAYO/MAEBBB5odHRwczovL2dpdGh1Yi5jb20vbG9naW4vb2F1dGgwgYoGCisG
AQQB1nkCBAIEfAR6AHgAdgDdPTBqxscRMmMZHhyZZzcCokpeuN48rf+HinKALynu
jgAAAYW5j/TGAAAEAwBHMEUCIQDwKj93qIqj8Cx5hP/ysY5v2jlPZHiXALFDll43
z45HGQIgLiJi0nXo3qdBrnGkrr71jI+EiEtLbDG4kiqqkO+oXZcwCgYIKoZIzj0E
AwMDZwAwZAIwJAdhcu5gF6kks5U/giqgyshGRVkz4/i99n64EC3qgl+XKe4THsZJ
xH7Yfv2WSAFiAjB/hs2fJC+tVndrCN7o6gaZp8lWgsfCtygXxHvZ3y5JW8nhqYAI
wCz0ypm111rsbAw=
-----END CERTIFICATE-----

tlog entry created with index: 11275062
Bundle wrote in the file artifact.bundle
MEUCIQDenV8e18eK3iaD05zEcJ2jjlWzpDG6bRaQzZrEymSCuwIgdqElIl38QqZXT9OxbZGQqWKSk1W5hJH3rScDcdSislE=
```
So this has produced a signature of the crate, the tar file. The crate would
then be published to crates.io and available to consumers.

Now, lets say I'm a malicious hacker and I run `cargo package` and then update
the .crate file with modified sources, or just craft my own tar file. I would
still be able to sign the .crate and publish it. A consumer would have no reason
not to trust this crate and verification of it would still pass.

For this situation a like this one can use a trusted CI builder, for example
a system that supports OIDC workload identities. These can verify the sources
and assurance that the sources have not been modified.


### Print git project hash
There is a utility program in this project that accepts a github https url and
prints the Cargo hash for that project. The intention is to show how the
directory names in `.cargo/git/` are created. For example, the directory
`~/.cargo/git/db/sigstore-rs-874f7064c0c10336/`, has a hash appended to it and
it might not be obvious where this came from, but it is a hash of the github
url. 

Usage:
```console
$ cargo r --quiet --bin project-hash -- -u https://github.com/sigstore/sigstore-rs.git
https://github.com/sigstore/sigstore-rs.git: 874f7064c0c10336
```

### Print Cargo index hash
The sole purpose of this tool is to print the hash appended to directories
`.cargo/registry/cache`, `.cargo/registry/index`, and `.cargo/registry/src`.
For example:
```console
$ ls ~/.cargo/registry/src/
github.com-1ecc6299db9ec823
```
Usage:
```console
$ cargo r --quiet --bin index-dir-hash
crates-io: 1ecc6299db9ec823
```
