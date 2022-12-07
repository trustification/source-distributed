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


### in-toto-rs
Now, to get this working we have been using Python to generate the in-toto root
layout file. There is [in-toto-rs](https://github.com/in-toto/in-toto-rs) which
would enable us to replace the Python with Rust to make this more targeted
towards Rust projects. So lets try it out.

So we started out by looking into the examples and trying to replicate what the
Python script does. We ran into an issue with in-toto-rs not having support
for `ecdsa` keys which have attempted to add in the [add link to pr here]().
Another issue was that the layout generated was not accepted by `in-toto-verify`
and the following error would be displayed:
```console
in-toto-verify -v -t ecdsa --layout source-distributed-layout.json --layout-keys=cosign.key.pub.jsonpwd
Loading layout...
(in-toto-verify) FormatError: Invalid Metadata format
```
There was already a pull request that sorted this out
[#41](https://github.com/in-toto/in-toto-rs/pull/41). This also introduced a
number of improvements with regards to how the layout is generated which meant
that we had to write some of the existing code. 

With that done we can run the `workflow.sh` scripts replacing the call to
`create_layout.py` with a call to [create-layout.rs](./src/create-layout.rs)
(Below we are just showing the call to `in-toto-verify`):
```console
$ in-toto-verify -v -t ecdsa --layout source-distributed-layout.json --layout-keys=cosign.key.pub.json
Loading layout...
Loading layout key(s)...
Verifying layout signatures...
(in-toto-verify) SignatureVerificationError: No signature found for key '67697271397ace039b7d1b3df0ca6a20e35c7bda160866555f116ff3deba4b1c'
```
Now, this looks like the same issue we encountered before and which we worked
around by converting the keys to the securesystemslib json format and avoid the
incorreclty generated keyids issue.

### create-layout.rs priv_key_from_pem issue
I'm currently troubleshooting an issue with how create-layout.rs which is the
reason for this section.

This is the json format that gets generated by the in-toto-key-import script
which is in the securesystemslib json format, which is the format that the
in-toto command line tools expect a ecdsa key to be in.
For example, it can look like this:
```console
$ cat ../cosign.key.json  | jq
{
  "keytype": "ecdsa",
  "scheme": "ecdsa-sha2-nistp256",
  "keyid": "67697271397ace039b7d1b3df0ca6a20e35c7bda160866555f116ff3deba4b1c",
  "keyval": {
    "public": "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEY62fGO3T7D69Hmu58+/QcDAXB30W\nzh84kXRBNviAkNyUf5hVVXcH/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END PUBLIC KEY-----\n",
    "private": "-----BEGIN PRIVATE KEY-----\nMHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49\nAwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH\n/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==\n-----END PRIVATE KEY-----\n"
  },
  "keyid_hash_algorithms": [
    "sha256",
    "sha512"
  ]
}
```
If we extract the `private` key and just focus on it, which is in pem format
and base64 encoded:
```console
-----BEGIN PRIVATE KEY-----
MHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49
AwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH
/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==
-----END PRIVATE KEY-----
```
If we paste that into a file and lets call it priv.pem then try reading this
file with openssl to see the output:
```console
$ openssl ec -in priv.pem -inform=pem -noout
read EC key
unable to load Key
140529587506688:error:0D0680A8:asn1 encoding routines:asn1_check_tlen:wrong tag:crypto/asn1/tasn_dec.c:1149:
140529587506688:error:0D07803A:asn1 encoding routines:asn1_item_embed_d2i:nested asn1 error:crypto/asn1/tasn_dec.c:309:Type=X509_ALGOR
140529587506688:error:0D08303A:asn1 encoding routines:asn1_template_noexp_d2i:nested asn1 error:crypto/asn1/tasn_dec.c:646:Field=pkeyalg, Type=PKCS8_PRIV_KEY_INFO
140529587506688:error:0907B00D:PEM routines:PEM_read_bio_PrivateKey:ASN1 lib:crypto/pem/pem_pkey.c:88:
```
Now, this fails due to an incorrect tag. If we update `priv.pem` and add the
`EC` to the header and footer:
this instead:
```console
-----BEGIN EC PRIVATE KEY-----                                                     
MHcCAQEEIFjdJCw6Lkx1VPYtdnbihRKoLFb36zzAc0XgCJ1B5/oPoAoGCCqGSM49                   
AwEHoUQDQgAEY62fGO3T7D69Hmu58+/QcDAXB30Wzh84kXRBNviAkNyUf5hVVXcH                   
/FwFtJ6S7P7snrB9BrhLWRIG7X6POF2CJw==                                            
-----END EC PRIVATE KEY-----
```
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
directly by cosign it works:
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
So the contents of this file is base64 encoded (without padding by the looks
of it).

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
