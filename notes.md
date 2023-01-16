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

We wrote a [program](./src/keygen.rs) in Rust that uses
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
I've created a [sigstore-rs/pull/#165](https://github.com/sigstore/sigstore-rs/pull/165) which a
suggestion about changing this tag and see what they think about it. That pull
request has now been closed without merging it and issue [#2471](https://github.com/sigstore/cosign/issues/2471)
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
example it might be a `git` dependency, or a `crates.io` dependency, it will
perform the verification is different ways.

For example a git dependency could be specified like this in Cargo.toml: 
```toml
source-distributed = { git = "ssh://git@github.com/trustification/source-distributed.git", branch="main" }
```
This information is used by `cargo-verify` to find the git repository that
cargo has checked out for the branch `main`. This location will be used to find
the expected artifacts needed to perform the verification.

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
add support for tags, and revisions. 

#### crates.io dependencies
So far we have worked out how we might work with git dependencies (only
supporting branches for now), so lets take a look at how we could do the same
thing but with a dependency from crates.io.

The local dependencies from crates.io are located in `/.cargo/registry`:
```console
$ ls ~/.cargo/registry/
cache  CACHEDIR.TAG  index  src
```
There can be multiple registries which are located in the index directory:
```
$ ls ~/.cargo/registry/index/
github.com-1ecc6299db9ec823
```
Now this was a little confusing to me as I did not expect a github.com directory
here. It turns out that Cargo communicates with registries through a github
repository which is called the `Index`. One such github repository is
https://github.com/rust-lang/crates.io-index.

Lets clone this index and take a look at it:
```console
$ git clone https://github.com/rust-lang/crates.io-index.git
$ cd crates.io-index/
```
If we list the contents of this directory we will see a number of subdirectories
starting with one or two characters/symbols/numbers. And there is also a
`config.json` file.

Now, notice that this index does not contain any crates:
```console
$ find . -name '*.crate' | wc -l
0
```
Instead what the index stores is a list of versions for all known packages. Each
crate will have a single file and there will be an entry in this file forward
each version.

Lets take a look at the `drg` crate:
```console
$ cat 3/d/drg 
{"name":"drg","vers":"0.1.0","deps":[],"cksum":"c6bfa8b0b1bcd485d5f783e77faf13ba9453e7ab78991936e50d6cfdca23d647","features":{},"yanked":true}
{"name":"drg","vers":"0.2.1","deps":[{"name":"anyhow","req":"^1.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"chrono","req":"^0.4","features":["serde"],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"clap","req":"^2.33.3","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"oauth2","req":"^3.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"qstring","req":"^0.7.2","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"reqwest","req":"^0.11","features":["blocking","json"],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"serde","req":"^1.0","features":["derive"],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"serde_json","req":"^1.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"strum","req":"^0.20","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"strum_macros","req":"^0.20","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"tempfile","req":"^3.2.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"tiny_http","req":"^0.8.0","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal"},{"name":"url","req":"^2.2.1","features":["serde"],"optional":false,"default_features":true,"target":null,"kind":"normal"}],"cksum":"cfb067bfabd64c3b4732a3afd2b9a757a88120f6dac6400eae5b865732be0404","features":{},"yanked":false}
...
```
Notice that there are three directories named `1`, `2`, and `3` which are for
crates that have one, two, or three letters/characters in their name. This is
the case with `drg` above.  

For other crates with longer names, the first directory matches the first two
characters of the crate, and the subdirectory under that will have another
directory matching the following two characters of the crate name. 
For example, if we want to find the `drogue-device` crate we would search for
`dr` as the first directory, and then `og` as the subdirectory:
```console
$ cat ./dr/og/drogue-device | jq
{
  "name": "drogue-device",
  "vers": "0.0.0",
  "deps": [],
  "cksum": "2acc1a9827b5cd933ebef9824415789012f5202b6bcacddaae2c214486ac996a",
  "features": {},
  "yanked": false
}
```
When new versions of this crate are released a new entry/line in this file will
be created. 

Updates to the index are fairly cheap, just like a normal git fetch and a
git fast forward. 

Alright, so we now have an effecient way to look up a crate version and its
dependencies but we haven't seen any crates yet.  This is where the file
`config.json` comes in to play:
```console
$ cat config.json 
{
  "dl": "https://crates.io/api/v1/crates",
  "api": "https://crates.io"
}
```
`dl` stands for `download` and is the url that can be used to download a
specific crate to the `.cargo/registry/github.com-1ecc6299db9ec823` directory.
```console
$ curl -v -L https://crates.io/api/v1/crates/drg/0.1.0/download --output drg-0.0.1.crate
```
And we should then be able to list the content of this crate:
```console
$ tar tvf drg-0.0.1.crate 
-rw-r--r-- 0/0              74 2021-03-18 15:57 drg-0.1.0/.cargo_vcs_info.json
-rw-r--r-- 110147/110147     8 2021-03-18 15:55 drg-0.1.0/.gitignore
-rw-r--r-- 0/0             134 2021-03-18 15:57 drg-0.1.0/Cargo.lock
-rw-r--r-- 0/0             754 2021-03-18 15:57 drg-0.1.0/Cargo.toml
-rw-r--r-- 110147/110147   327 2021-03-18 15:56 drg-0.1.0/Cargo.toml.orig
-rw-r--r-- 110147/110147    45 2021-03-18 15:55 drg-0.1.0/src/main.rs
```
So the `.cargo/registry/cache/github.com-1ecc6299db9ec823` will only contain the
downloaded crates, the `.crate` compressed tar files. These never change for a
versions so they don't have to be downloaded again.

We are interested in the `src` directory which is the directory into which
the download crates in the cache director are unpacked:
```console
$ ls ~/.cargo/registry/src/
github.com-1ecc6299db9ec823
```

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


### in-toto-rs
Now, to get this working we have been using Python to generate the in-toto root
layout file. There is [in-toto-rs](https://github.com/in-toto/in-toto-rs) which
would enable us to replace the Python with Rust to make this more targeted
towards Rust projects. So lets try it out.

So we started out by looking into the examples and trying to replicate what the
Python script does. We ran into an issue with in-toto-rs not having support
for `ecdsa` keys which have attempted to add in this [PR](https://github.com/in-toto/in-toto-rs/pull/44).

Initially we used the private key generated by cosign directly to sign the
layout and this work. The key format of these keys are in PKCS8. But in-toto's
command line tools expect keys to be in securesystemslib json format. In this
format the public/private keys are part for the `keyval` element:
```
{
  "keytype": "ecdsa",
  "scheme": "ecdsa-sha2-nistp256",
  "keyid": "67697271397ace039b7d1b3df0ca6a20e35c7bda160866555f116ff3deba4b1c",
  "keyval": {
    "public": "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEY62fGO3T7D69Hmu58+/QcDAXB30W\nzh84kXRBNviAkNyUf5hVVXcH/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END PUBLIC KEY-----\n",
    "private": "-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49\nAwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH\n/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END EC PRIVATE KEY-----\n"
  },
  "keyid_hash_algorithms": [
    "sha256",
    "sha512"
  ]
}
```
Notice that the private key in this case is not in pkcs8 format (the general
format which can hold various types for private keys) but instead in EC Private
Key format. For some further details on these format see
[key-formats.md](https://github.com/danbev/learning-crypto/blob/main/notes/key-formats.md).

While we have suggested adding [ecdsa support](https://github.com/in-toto/in-toto-rs/pull/44)
to in-toto-rs this was only done for pkcs8 keys. While the underlying key
information is the same, the different formatting has created some issues for
us.
We are currently working on adding support to in-toto-rs to accept the above
json format, parse the contents of the private and public key and use them
to generate the in-toto-rs specific `PrivateKey`. In the process we would also
like to be able to use the `keyid` from the above json to avoid issues with
verifying later.
For example:
```rust
    let priv_key = PrivateKey::from_securesystemslib_ecdsa(s).unwrap();
```


### create-layout.rs priv_key_from_pem issue
I'm currently troubleshooting an issue with how create-layout.rs which is the
reason for this section.

This is the json format that gets generated by the in-toto-key-import script
which is in the securesystemslib json format. Recall that this is the format
that in-toto expects the key to be in.

For example, it can look like this:
```console
$ cat ../cosign.key.json  | jq
{
  "keytype": "ecdsa",
  "scheme": "ecdsa-sha2-nistp256",
  "keyid": "67697271397ace039b7d1b3df0ca6a20e35c7bda160866555f116ff3deba4b1c",
  "keyval": {
    "public": "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEY62fGO3T7D69Hmu58+/QcDAXB30W\nzh84kXRBNviAkNyUf5hVVXcH/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END PUBLIC KEY-----\n",
    "private": "-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49\nAwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH\n/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END EC PRIVATE KEY-----\n"
  },
  "keyid_hash_algorithms": [
    "sha256",
    "sha512"
  ]
}
```
If we extract the `private` key and just focus on it:
```console
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49
AwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH
/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==
-----END EC PRIVATE KEY-----
```
We can inspect this private key using openssl:
```console
$ openssl ec -in priv.pem -inform=pem -noout --text
read EC key
Private-Key: (256 bit)
priv:
    58:dd:24:2c:3a:2e:4c:75:54:f6:2d:76:76:e2:85:
    12:a8:2c:56:f7:eb:3c:c0:73:45:e0:08:9d:41:e7:
    fa:0f
pub:
    04:63:ad:9f:18:ed:d3:ec:3e:bd:1e:6b:b9:f3:ef:
    d0:70:30:17:07:7d:16:ce:1f:38:91:74:41:36:f8:
    80:90:dc:94:7f:98:55:55:77:07:fc:5c:05:b4:9e:
    92:ec:fe:ec:9e:b0:7d:06:b8:4b:59:12:06:ed:7e:
    8f:38:5d:82:27
ASN1 OID: prime256v1
NIST CURVE: P-256
```
So the same information is in both of these and if we use the key produced
directly by cosign it works.

So the differences is that the key produced by cosign is in pkcs8 format and
the key in the securesystemslib json is in Elliptic Curve Private Key format.
These are different ways of representing a key, pkcs8 is more general and can
hold different types of keys which is the reason it has an object identifier.


```console
$ cat ../cosign.key
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWN0kLDouTHVU9i12
duKFEqgsVvfrPMBzReAInUHn+g+hRANCAARjrZ8Y7dPsPr0ea7nz79BwMBcHfRbO
HziRdEE2+ICQ3JR/mFVVdwf8XAW0npLs/uyesH0GuEtZEgbtfo84XYIn
-----END PRIVATE KEY-----

$ openssl ec -in ../cosign.key -inform=pem -noout --text
read EC key
Private-Key: (256 bit)
priv:
    58:dd:24:2c:3a:2e:4c:75:54:f6:2d:76:76:e2:85:
    12:a8:2c:56:f7:eb:3c:c0:73:45:e0:08:9d:41:e7:
    fa:0f
pub:
    04:63:ad:9f:18:ed:d3:ec:3e:bd:1e:6b:b9:f3:ef:
    d0:70:30:17:07:7d:16:ce:1f:38:91:74:41:36:f8:
    80:90:dc:94:7f:98:55:55:77:07:fc:5c:05:b4:9e:
    92:ec:fe:ec:9e:b0:7d:06:b8:4b:59:12:06:ed:7e:
    8f:38:5d:82:27
ASN1 OID: prime256v1
NIST CURVE: P-256
```
If I replace the contents of json private field with the contents from
cosign.key then it works as expected (just to rule out anything else that migth
be causing issues).

If we base64 decode the contents for cosign.pem, which is what we have in the
json the we get:
```console
$ cat b.pem | base64 -d - > output1
$ ls -l output1
-rw-r--r--. 1 danielbevenius danielbevenius 121 Dec  7 09:12 output1

$ openssl ec -inform der -in output1 -outform pem -text
read EC key
Private-Key: (256 bit)
priv:
    58:dd:24:2c:3a:2e:4c:75:54:f6:2d:76:76:e2:85:
    12:a8:2c:56:f7:eb:3c:c0:73:45:e0:08:9d:41:e7:
    fa:0f
pub:
    04:63:ad:9f:18:ed:d3:ec:3e:bd:1e:6b:b9:f3:ef:
    d0:70:30:17:07:7d:16:ce:1f:38:91:74:41:36:f8:
    80:90:dc:94:7f:98:55:55:77:07:fc:5c:05:b4:9e:
    92:ec:fe:ec:9e:b0:7d:06:b8:4b:59:12:06:ed:7e:
    8f:38:5d:82:27
ASN1 OID: prime256v1
NIST CURVE: P-256
writing EC key
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49
AwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH
/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==
-----END EC PRIVATE KEY-----
```

So we should be able to convert from the pem format, the string, and get
that into pkcs8 format, and then pass that to the function.
```
$ openssl ec -inform pem -in ../cosign.key -outform pem -out cosign.pem -text
$ openssl pkcs8 -in cosign.pem -out cosign.pkcs8 -topk8 -nocrypt
$ cat cosign.pkcs8 
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWN0kLDouTHVU9i12
duKFEqgsVvfrPMBzReAInUHn+g+hRANCAARjrZ8Y7dPsPr0ea7nz79BwMBcHfRbO
HziRdEE2+ICQ3JR/mFVVdwf8XAW0npLs/uyesH0GuEtZEgbtfo84XYIn
-----END PRIVATE KEY-----
```
So we need to take the private key generated in pem format and convert it into
pkcs8 format, and then pass that to in-toto-rs. We can use a function in
openssl-rs for this which is what I've opted for now atleast. 

So that will allow the create-layout.rs to pass and sign the layout, but we
still have an issue with the generated keyid's which is a re-occuring theme.
There should really be a way to specify the keyid that should be used and not
have these tied to the contens of the pems.

This is the Elliptic Curve Private Key format:
```
[30, 77, 02, 01, 01, 04, 20, 58, dd, 24, 2c, 3a, 2e, 4c, 75, 54, f6, 2d, 76, 76, e2, 85, 12, a8, 2c, 56, f7, eb, 3c, c0, 73, 45, e0, 08, 9d, 41, e7, fa, 0f, a0, 0a, 06, 08, 2a, 86, 48, ce, 3d, 03, 01, 07, a1, 44, 03, 42, 00, 04, 63, ad, 9f, 18, ed, d3, ec, 3e, bd, 1e, 6b, b9, f3, ef, d0, 70, 30, 17, 07, 7d, 16, ce, 1f, 38, 91, 74, 41, 36, f8, 80, 90, dc, 94, 7f, 98, 55, 55, 77, 07, fc, 5c, 05, b4, 9e, 92, ec, fe, ec, 9e, b0, 7d, 06, b8, 4b, 59, 12, 06, ed, 7e, 8f, 38, 5d, 82, 27]

$ openssl asn1parse -in cosign.pem

    0:d=0  hl=2 l= 119 cons: SEQUENCE          
    2:d=1  hl=2 l=   1 prim: INTEGER           :01
    5:d=1  hl=2 l=  32 prim: OCTET STRING      [HEX DUMP]:58DD242C3A2E4C7554F62D7676E28512A82C56F7EB3CC07345E0089D41E7FA0F
   39:d=1  hl=2 l=  10 cons: cont [ 0 ]        
   41:d=2  hl=2 l=   8 prim: OBJECT            :prime256v1
   51:d=1  hl=2 l=  68 cons: cont [ 1 ]        
   53:d=2  hl=2 l=  66 prim: BIT STRING
```
The [format](https://www.rfc-editor.org/rfc/rfc5915#section-3) looks like this:
```
ECPrivateKey ::= SEQUENCE {
     version        INTEGER { ecPrivkeyVer1(1) } (ecPrivkeyVer1),
     privateKey     OCTET STRING,
     parameters [0] ECParameters {{ NamedCurve }} OPTIONAL,
     publicKey  [1] BIT STRING OPTIONAL
}
```

And if we compare this with the cosign.key what is generated by cosign it looks
like this:
```
[30, 81, 87, 02, 01, 00, 30, 13, 06, 07, 2a, 86, 48, ce, 3d, 02, 01, 06, 08, 2a, 86, 48, ce, 3d, 03, 01, 07, 04, 6d, 30, 6b, 02, 01, 01, 04, 20, 58, dd, 24, 2c, 3a, 2e, 4c, 75, 54, f6, 2d, 76, 76, e2, 85, 12, a8, 2c, 56, f7, eb, 3c, c0, 73, 45, e0, 08, 9d, 41, e7, fa, 0f, a1, 44, 03, 42, 00, 04, 63, ad, 9f, 18, ed, d3, ec, 3e, bd, 1e, 6b, b9, f3, ef, d0, 70, 30, 17, 07, 7d, 16, ce, 1f, 38, 91, 74, 41, 36, f8, 80, 90, dc, 94, 7f, 98, 55, 55, 77, 07, fc, 5c, 05, b4, 9e, 92, ec, fe, ec, 9e, b0, 7d, 06, b8, 4b, 59, 12, 06, ed, 7e, 8f, 38, 5d, 82, 27]

$ openssl asn1parse -in ../cosign.key
    0:d=0  hl=3 l= 135 cons: SEQUENCE          
    3:d=1  hl=2 l=   1 prim: INTEGER           :00
    6:d=1  hl=2 l=  19 cons: SEQUENCE          
    8:d=2  hl=2 l=   7 prim: OBJECT            :id-ecPublicKey
   17:d=2  hl=2 l=   8 prim: OBJECT            :prime256v1
   27:d=1  hl=2 l= 109 prim: OCTET STRING      [HEX DUMP]:306B020101042058DD242C3A2E4C7554F62D7676E28512A82C56F7EB3CC07345E0089D41E7FA0FA1440342000463AD9F18EDD3EC3EBD1E6BB9F3EFD0703017077D16CE1F3891744136F88090DC947F9855557707FC5C05B49E92ECFEEC9EB07D06B84B591206ED7E8F385D8
```
This is in [pkcs8](https://datatracker.ietf.org/doc/html/rfc5958)
format: 
```
PrivateKeyInfo ::= SEQUENCE {
  version                   Version,
  privateKeyAlgorithm       PrivateKeyAlgorithmIdentifier,
  privateKey                PrivateKey,
  attributes           [0]  IMPLICIT Attributes OPTIONAL
}

PrivateKeyAlgorithmIdentifier ::= AlgorithmIdentifier
PrivateKey ::= OCTET STRING
Attributes ::= SET OF Attribute

AlgorithmIdentifier ::= SEQUENCE {
  algorithm       OBJECT IDENTIFIER,
  parameters      ANY DEFINED BY algorithm OPTIONAL
}
```
The version is `00` which the spec says must be 0 for this version.
The PrivateKeyAlgorithmIdentifier is expaneded to something that identifies the
type of private key in this sequence/struct. pkcs8 can be used with many
different types of private keys which is why this is here.

I think that `id-ecPublicKey` is actually an identifier/name for the Object
Identifier 1.2.840.10045.2.1.

Now, with addition of [support for ecdsa keys](https://github.com/in-toto/in-toto-rs/pull/44)
we are now able to sign the layout with the ecdsa private key. But we still have
the issue with the keyid from the bundle.json not matching that of the private
key.

If we take a look at the call to`from_pkcs8`, it looks like this:
```rust
    let priv_key = PrivateKey::from_pkcs8(&der.contents, SignatureScheme::EcdsaP256Sha256).unwrap();
```
And the implentation for `ecdsa_from_pkcs8` looks like this:
```rust
    fn ecdsa_from_pkcs8(der_key: &[u8], scheme: SignatureScheme) -> Result<Self> {
        let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, der_key).unwrap();
        let public = PublicKey::new(
            KeyType::Ecdsa,
            scheme,
            python_sslib_compatibility_keyid_hash_algorithms(),
            key_pair.public_key().as_ref().to_vec(),
        )?;
        let private = PrivateKeyType::Ecdsa(key_pair);
        Ok(PrivateKey { private, public })
    }
```
The keyid is in the PublicKey struct:
```rust
  pub struct PublicKey {
      typ: KeyType,
      key_id: KeyId,
      scheme: SignatureScheme,
      keyid_hash_algorithms: Option<Vec<String>>,
      value: PublicKeyValue,
  }

 impl PublicKey {
  fn new(
          typ: KeyType,
          scheme: SignatureScheme,
          keyid_hash_algorithms: Option<Vec<String>>,
          value: Vec<u8>,
      ) -> Result<Self> {
      let key_id = calculate_key_id(&typ, &scheme, &keyid_hash_algorithms, &value)?;
      let value = PublicKeyValue(value);
      Ok(PublicKey {
          typ,
          key_id,
          scheme,
          keyid_hash_algorithms,
          value,
      })
  }
```
Notice the key_id is generated by calling `calculate_key_id` passing in the
KeyType, the SignatureScheme, the optional keyid_hash_algoritms, and the
bytes of the public key itself.
```console
PublicKey::new type: Ecdsa, scheme EcdsaP256Sha256, keyid_hash_algorithms: Some(["sha256", "sha512"]), value: [4, 99, 173, 159, 24, 237, 211, 236, 62, 189, 30, 107, 185, 243, 239, 208, 112, 48, 23, 7, 125, 22, 206, 31, 56, 145, 116, 65, 54, 248, 128, 144, 220, 148, 127, 152, 85, 85, 119, 7, 252, 92, 5, 180, 158, 146, 236, 254, 236, 158, 176, 125, 6, 184, 75, 89, 18, 6, 237, 126, 143, 56, 93, 130, 39]
```

And if we look at the `calculate_key_id` function we find the following:
```rust
  fn calculate_key_id(
      key_type: &KeyType,
      signature_scheme: &SignatureScheme,
      keyid_hash_algorithms: &Option<Vec<String>>,
      public_key: &[u8],
  ) -> Result<KeyId> {
      use crate::interchange::{DataInterchange, Json};

      let public_key = shim_public_key(
          key_type,
          signature_scheme,
          keyid_hash_algorithms,
          public_key,
          false,
          None,
      )?;
      let public_key = Json::canonicalize(&Json::serialize(&public_key)?)?;
      let public_key = String::from_utf8(public_key)
          .map_err(|e| Error::Encoding(format!("public key from bytes to string failed: {}", e,)))?
          .replace("\\n", "\n");
      let mut context = digest::Context::new(&SHA256);
      context.update(public_key.as_bytes());

      let key_id = HEXLOWER.encode(context.finish().as_ref());

      Ok(KeyId(key_id))
  }
```
Notice that the last argument to `shim_public_key` is `None` and that this is
the keyid which optional. `shim_public_key` is a function that returns a
shims::PublicKey:
```rust
  fn shim_public_key(
      key_type: &KeyType,
      signature_scheme: &SignatureScheme,
      keyid_hash_algorithms: &Option<Vec<String>>,
      public_key: &[u8],
      private_key: bool,
      keyid: Option<&str>,
  ) -> Result<shims::PublicKey> {
      let key = match key_type {
          ...
          KeyType::Ecdsa => HEXLOWER.encode(public_key),
      };

      let private_key = match private_key {
          true => Some(""),
          false => None,
      };

      Ok(shims::PublicKey::new(
          key_type.clone(),
          signature_scheme.clone(),
          keyid_hash_algorithms.clone(),
          key,
          keyid,
          private_key,
      ))
  }
```
This shims::PublicKey is a struct that defines some serdes rules.
```rust
#[derive(Serialize, Deserialize)]
pub struct PublicKey {
    keytype: crypto::KeyType,
    scheme: crypto::SignatureScheme,

    #[serde(skip_serializing_if = "Option::is_none")]
    keyid_hash_algorithms: Option<Vec<String>>,

    keyval: PublicKeyValue,

    #[serde(skip_serializing_if = "Option::is_none")]
    keyid: Option<String>,
}
```

The keyid from the bundle file is:
```
keyid:  "67697271397ace039b7d1b3df0ca6a20e35c7bda160866555f116ff3deba4b1c"
```
And the generated keyid is:
```
keyid: KeyId("18e9bb5af7fd2f6ba004377bd872eaae19b2c956cb7bbdbfd7f1b35a5cba9a73")

```

So this is what will be hashed and used as the key_id:
```console
"{\"keyid_hash_algorithms\":[\"sha256\",\"sha512\"],\"keytype\":\"ecdsa\",\"keyval\":{\"public\":\"0463ad9f18edd3ec3ebd1e6bb9f3efd0703017077d16ce1f3891744136f88090dc947f9855557707fc5c05b49e92ecfeec9eb07d06b84b591206ed7e8f385d8227\"},\"scheme\":\"ecdsa-sha2-nistp256\"}"
```
And the generated hash will be:
```
hash (key_id): "32c305141e442ad9ea7aae6695cd3282aeb018ee363781c066ad826c1929245e
```

After thinking about this some more doing this conversion seems like a lot of
code and it would be better to allow in-toto-rs to accept the securesystemslib
json format and generate the keys from that.

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

### Generate in-toto artifacts
Use the following command to generate the in-toto artifacts
```console
$ cargo r --bin cargo-in-toto-gen -- -o trustification -r source-distributed
```

Use the following command to verify a source dependency:
```console
$ cargo r --bin cargo-verify -- -d source-distributed
```

The following option can be used to check a directory that is outside of
`~/.cargo/git`:
```console
$ cargo r --bin cargo-verify -- -d source-distributed -a sscs/in-toto/artifacts/main -p $PWD

```
