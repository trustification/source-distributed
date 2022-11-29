## Source distributed projects
This document contains notes about an attempt to secure source distributed
projects, in particular smaller project that are often used as third-party
depencencies by larger project.

The motivation for this is that adding signing/attestations/sbom for our 
project is great, but a lot or project have a number of third-party dependencies
and their security level is unknown may be vulnerable to exploits which also
means that our projects security level is unknown in reality.

I might be naive here, but could we start helping open source projects, like the
ones that are most used by our customers and help them setup in-toto or
something else.

This document is a description/journal of the experience of trying this out.

## Using cosign keys for signing with in-toto
The goal here was to use cosign's ephemeral keys, and then use them with in-toto.
An additional goal was that this process not require any human interaction and
that it should be possible to run it as a github action. 

The first task was to setup a [github action](./github/workflows/sscs.yaml) that
uses githubs OIDC access token and pass that to Fulcio, Sigstore's Certificate
Authority (CA).

We wrote to [program](./src/keygen.rs) in Rust that uses
[sigstore-rs](https://github.com/sigstore/sigstore-rs) to request a
signing-certificate from Fulcio and save the short-lived keys and certificate
to disk. The keys types from Fulcio are `ecdsa` which in-toto did not have
support for in their command line tools. We created and issue for this addition,
[#519](https://github.com/in-toto/in-toto/issues/519), and the in-toto command
line tools now have support for these types.

Next we wanted to use these keys for creating the in-toto layout and also for
creating the steps. Now, as mentioned earlier the keys from cosign are of type
`ecdsa` and in-toto now also has support for `ecdsa` but in-toto requires that
the format of the `ecdsa` keys be in `securesystemslib` json format whereas the
keys from cosign are in pem format.

So we added a Python script to perform this conversion, python because in-toto
and securesystemslib is written in python and we could use methods provided.
But we ran into an
[issue](https://github.com/secure-systems-lab/securesystemslib/pull/457) with
how the method create the `keyid` field. We were able to work around this in the
script. As others might have the same need to doing this conversion we have
created an [issue](https://github.com/in-toto/in-toto/issues/522) in in-toto
suggesting something be created for doing this (and perhaps hide some of the
internal details regarding the json format).

With those changes we can now have a github action that creates an in-toto
layout which is signed, and also create the steps (currently only a git clone
and running of tests) and verify the layout. But this verification and signing
is not using the additional Sigstore components like Rekor, the transparency
log. The next step is to look into how this can be made possible.

So at this stage we have the generated in-toto artifacts which have been signed
using the ephemeral key. But we also want to sign and upload these artifacts to
Rekor so that they can later be verified.

So the command we use to verfify the in-toto layout is the following:
```console
$ in-toto-verify -v -t ecdsa --layout source-distributd-layout.json --layout-keys=cosign.pub
```
This will also use the three link files:
```console
$ ls *.link
cargo-fetch.link  clone_project.0e7e4a83.link  run_tests.0e7e4a83.link
```

We could tar these files and then sign that blob. Now if we try that we get
an error:
```console
$ cosign sign-blob --key cosign.key artifacts.tar 
Using payload from: artifacts.tar
Enter password for private key: 
Error: signing artifacts.tar: reading key: unsupported pem type: PRIVATE KEY
main.go:62: error during command execution: signing artifacts.tar: reading key: unsupported pem type: PRIVATE KEY
```
Notice that we are prompted for a password as we don't provide one when
requesting a signing certificate from Fulcio. But we can convert the private key
into a 

We can generate an encrypted key using `private_key_to_encrypted_pem` and then
store this as `cosign.key.enc`:
```console
$ cat cosign.key.enc 
-----BEGIN ENCRYPTED SIGSTORE PRIVATE KEY-----
eyJrZGYiOnsibmFtZSI6InNjcnlwdCIsInBhcmFtcyI6eyJOIjozMjc2OCwiciI6
OCwicCI6MX0sInNhbHQiOiJaN2NxN1R6OXJxd3pGZ0xOS3BXNjVYR25ZTXVPUWI3
VjlkZ3htc3RVNHVvPSJ9LCJjaXBoZXIiOnsibmFtZSI6Im5hY2wvc2VjcmV0Ym94
Iiwibm9uY2UiOiJBNGxmUDAyNXE2aW16T3hicTQ4Tk1vZlVRdjg2UFJDViJ9LCJj
aXBoZXJ0ZXh0IjoiWGxLUjdRZVJVUjM2endURWl3YzdDVytpZFdzYkF2U3dJZDRa
N2hiS3FocnQ1Z21xYWZwemU0MWlla2JrQ1RIbDdhbjBjZGhnays4SXloYVBTSVlK
MmFCWFZNYlgxVlZzY2NGL2p3eklVaHpKTnltdXNLRERGU1Fzd1Z4eStSd3UwejA0
R0FkcXpNNHNoenFQSzBhL1JSWWdsR01lcGtYbE9xZzNCUGVuQllqYk1SVDRrYm1h
cklFak94WDYxVjc1UldEdnBTTW5abG1WS3c9PSJ9
-----END ENCRYPTED SIGSTORE PRIVATE KEY-----
```
But we get the same `unsupported pem type`:
```console
$ cosign sign-blob -d --key cosign.key.enc artifacts.tar 
Using payload from: artifacts.tar
Enter password for private key: 
Error: signing artifacts.tar: reading key: unsupported pem type: ENCRYPTED SIGSTORE PRIVATE KEY
main.go:62: error during command execution: signing artifacts.tar: reading key: unsupported pem type: ENCRYPTED SIGSTORE PRIVATE KEY
```
It  seems like go cosign is wanting [ENCRYPTED COSIGN PRIVATE KEY](https://github.com/sigstore/cosign/blob/6b309df06f60ea5f58db22e9890713138c823d27/pkg/cosign/keys.go#L41) and not
`ENCRYPTED SIGSTORE PRIVATE KEY`. Just changing this in the pem allowed for the
command to succeed:
```console
$ env COSIGN_EXPERIMENTAL=1 cosign sign-blob -d --bundle artifacts.bundle --key cosign.key.enc artifacts.tar
Using payload from: artifacts.tar
Enter password for private key: 
tlog entry created with index: 7275006
Bundle wrote in the file artifacts.bundle
MEUCIQCiehDxhd4mSKgTRC43c4TX6FyNEm2Lks29s7EiqNX7TAIgez1+KWB2fNfZfNt/sDnqJ9solE+I1R9XhFdZl/BKkN8=
```
I've created a [PR](https://github.com/sigstore/sigstore-rs/pull/165) which a
suggestion about changing this tag and see what they think about it. That pull
request has now been closed with out merging it and issue [#2471](https://github.com/sigstore/cosign/issues/2471)
opened instead to allow `cosign` implementations to accept the SIGSTORE keys in
addition to COSIGN keys.

```console
$ env COSIGN_PASSWORD="_" cosign verify-blob --bundle=artifacts.bundle artifacts.tar
tlog entry verified offline
Verified OK
```

So keeping in mind that we are talking about source distributed projects and
a project would be using these as thirdparty dependencies.

### Client side usage
This section is an attempt to figure out how a client would verify a
dependency (that was secured by in-toto as described above).

A user or system could run the following command named
[cargo-verify](./src/bin/cargo-verify.rs):
```console
$ cargo r --bin cargo-verify -- -d source-distributed
```
The command line argument `-d` or `--depencency` specifies a dependency that
is expected to exist in `Cargo.toml`. Depending on the type of dependency, for
example it might be a `git` dependency or a `crates.io` dependency, it will
perform the verification is different ways. A git dependency has been chosen to
try this out: 
```toml
source-distributed = { git = "ssh://git@github.com/trustification/source-distributed.git", branch="main" }
```
This information is used by `cargo-verify` to find the git repository that
cargo has checked out for the branch main. This location will used to find the
expected artifacts to be verified.

The output of the above command will look like this:
```console
$ cargo r --bin cargo-verify -- -d source-distributed
   Compiling cargo v0.66.0
   Compiling source-distributed v0.1.0 (/home/danielbevenius/work/security/source-distributed)
    Finished dev [unoptimized + debuginfo] target(s) in 36.11s
     Running `target/debug/cargo-verify -d source-distributed`

Verifying dependency: source-distributed

git_db_path: /home/danielbevenius/.cargo/git/db/source-distributed-91fc664624018534
git_checkouts_path: /home/danielbevenius/.cargo/git/checkouts/source-distributed-91fc664624018534

Branch: main resolved to revision 393ad06

artifacts_dir: "/home/danielbevenius/.cargo/git/checkouts/source-distributed-91fc664624018534/393ad06/sscs/in-toto/artifacts"
artifact_tar: "/home/danielbevenius/.cargo/git/checkouts/source-distributed-91fc664624018534/393ad06/sscs/in-toto/artifacts/main.tar"

verify status: exit status: 0
verify stdout: 
verify stderr: Loading layout...
Loading layout key(s)...
Verifying layout signatures...
Verifying layout expiration...
Reading link metadata files...
Verifying link metadata signatures...
Verifying sublayouts...
Verifying alignment of reported commands...
Verifying command alignment for 'clone_project.2a949d99.link'...
Verifying command alignment for 'run_tests.2a949d99.link'...
Verifying threshold constraints...
Skipping threshold verification for step 'clone_project' with threshold '1'...
Skipping threshold verification for step 'run_tests' with threshold '1'...
Verifying Step rules...
Verifying material rules for 'clone_project'...
Verifying product rules for 'clone_project'...
Verifying 'CREATE source-distributed'...
Verifying 'ALLOW source-distributed/*'...
Verifying 'ALLOW source-distributed-layout.json'...
Verifying material rules for 'run_tests'...
Verifying 'MATCH source-distributed/* WITH PRODUCTS FROM clone_project'...
Verifying 'ALLOW Cargo.toml'...
Verifying 'DISALLOW *'...
Verifying product rules for 'run_tests'...
Verifying 'ALLOW Cargo.lock'...
Verifying 'ALLOW cosign.key.json'...
Verifying 'ALLOW cosign.key.pub.json'...
Verifying 'DISALLOW *'...
Executing Inspection commands...
Executing command for inspection 'cargo-fetch'...
Running 'cargo-fetch'...
Recording materials '.'...
Running command 'git clone git@github.com:trustification/source-distributed.git'...
Recording products '.'...
Creating link metadata...
Verifying Inspection rules...
Verifying material rules for 'cargo-fetch'...
Verifying 'MATCH source-distributed/* WITH PRODUCTS FROM clone_project'...
Verifying 'ALLOW source-distributed/target'...
Verifying 'ALLOW cosign.key.pub.json'...
Verifying 'ALLOW source-distributed-layout.json'...
Verifying 'DISALLOW *'...
Verifying product rules for 'cargo-fetch'...
Verifying 'MATCH source-distributed/Cargo.toml WITH PRODUCTS FROM clone_project'...
Verifying 'MATCH * WITH PRODUCTS FROM clone_project'...
Verifying 'ALLOW source-distributed/target'...
Verifying 'ALLOW cosign.key.pub.json'...
Verifying 'ALLOW cosign.key.pub.json'...
Verifying 'ALLOW source-distributed-layout.json'...
The software product passed all verification.

```

#### git dependencies
Something that has confused me in the past when looking into the 
`.cargo/git/db`, and `.cargo/git/checkouts` directories was the hash appended to
 the repository names. For example:
```
source-distributed-91fc664624018534
```
This is a hash of the url of the git repository, and is something that we needed
when implementing `cargo-verify`.

The directories in `.cargo/git/db` are the bare git repositories, and the
directories in `.cargo/git/checkouts` are the checked out revisions which have
a directory for each revision (short hash) used by Cargo.

The implementation here is just to try to iron out how this might work and get
a feel for things but at the same time actually going through the steps to
discover issues. At the moment only git branches are supported, but we could
add support for tags, revisions. And also there has not been any work done for
dependencies coming from crates.io (yet).

#### crates.io dependencies
So far we have worked out how we might work with git dependencies (only
supporting branches for now), so lets take a look at how we could do the same
thing but with a dependency from crates.io.

The dependencies from crates.io are located in `/.cargo/registry`:
```console
$ ls ~/.cargo/registry/
cache  CACHEDIR.TAG  index  src
```
There can be multiple registries which are located in the index directory:
```
$ ls ~/.cargo/registry/index/
github.com-1ecc6299db9ec823
```

We are interested in the `src` directory:
```console
$ ls ~/.cargo/registry/src/
github.com-1ecc6299db9ec823
```
Now this was a little confusing to me as I did not expect a github.com directory
here. It turns out that Cargo communicates with registries through a github
repository which is called the `Index`.
One such github repository can is https://github.com/rust-lang/crates.io-index.
The hash following the host is the hash of a `SourceId` instance:
```rust
    let registry_id = SourceId::crates_io(&config).unwrap();
    let host = registry_id.url().host().unwrap().to_string();
    let dir_name = format!("{}-{}", host, cargo::util::hex::short_hash(&registry_id));
```
The above will produce:
```console
github.com-1ecc6299db9ec823
```
So that gives us access to the source directory. And to find the specified
dependency we can use the dependency name and the version:
```console
    let dep_dir_name = format!("{}-{}", dependency_name, version);
```
With this information we have a path to the unpacked crate and can try to
verify that directory using the same function that was used to verify the git
directory.

_work in progress_

### Running the workflow locally
The same workflow that the github action runs can be run locally, in which case
there the OIDC flow will open a browser to choose the OICD Provider to use:
```console
$ cd sscs/in-toto
$ ./workflow
```
The output of the command will then be available in
[artifacts](./sscs/in-toto/artifacts).

### Running the keygen tool
The [keygen](./src/keygen.rs) tool can be run by itself using the following
command:
```console
$ cargo r --bin keygen
```
This will generate three files, `cosign.key`, `cosign.pub`, and `cosign.crt`.
