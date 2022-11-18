#!/bin/bash
## This script is intended to be run manually and simulate the process of
# creating in-toto steps and layout.
#
# Output will be produced in the artifacts directory which includes the link
# files, the layout, and the public key (in "json" format).
GITHUB_ORG="${1:-trustification}"
GITHUB_PROJECT="${2:-source-distributed}"
GITHUB_TOKEN=$3
PRIVATE_KEY=cosign.key
PUBLIC_KEY=${PRIVATE_KEY}.pub.json

artifacts_dir="artifacts"
artifacts_tar="${artifacts_dir}.tar"
verify_dir="verify"

echo "workflow.sh arguments: $GITHUB_ORG $GITHUB_PROJECT $GITHUB_TOKEN"

## First generate the keypair to be used when signing
echo "Generate keypair"
if [ -z $GITHUB_TOKEN ]; then
	echo "without token...."
	cargo r --manifest-path=../../Cargo.toml --bin keygen
else
	echo "with token...."
	cargo r --manifest-path=../../Cargo.toml --bin keygen $GITHUB_TOKEN
fi

# Change the pem tag to BEGIN ENCRYPTED COSIGN PRIVATE KEY, releated to
# https://github.com/sigstore/sigstore-rs/pull/165
sed -i 's/SIGSTORE/COSIGN/' cosign.key.enc

# Create keys in securesystemslib json key format. This will generate two
# files: <keyname>.key.json and <keyname>.key.pub.json
echo "Import and convert cosign keys to securesystemslib json format"
./in-toto-key-import.py $PRIVATE_KEY

mkdir -p $artifacts_dir
echo "Create layout"
./create-layout.py $GITHUB_ORG $GITHUB_PROJECT ${PRIVATE_KEY}.json $PUBLIC_KEY

echo "Create steps"
./create-steps.sh $GITHUB_ORG $GITHUB_PROJECT ${PRIVATE_KEY}.json $PUBLIC_KEY

echo "Tar the artifacts"
pushd $artifacts_dir > /dev/null
tar -cvf $artifacts_tar *
mv $artifacts_tar ../
popd > /dev/null

echo "Sign the $artifacts_tar with cosign"
env COSIGN_PASSWORD="_" COSIGN_EXPERIMENTAL=1 cosign sign-blob -d --bundle artifacts.bundle --output-certificate=artifacts.crt --output-signature=artifacts.sig --key cosign.key.enc $artifacts_tar

echo "Verify $artifacts_tar with cosign"
env COSIGN_EXPERIMENTAL=1 cosign verify-blob --bundle=artifacts.bundle $artifacts_tar

mkdir -p ${verify_dir}
cp $artifacts_tar $verify_dir
echo "Verify the contents of $artifacts_tar"
pushd $verify_dir > /dev/null
tar xf $artifacts_tar
rm $artifacts_tar
in-toto-verify -v -t ecdsa --layout $GITHUB_PROJECT-layout.json --layout-keys=$PUBLIC_KEY
popd > /dev/null
rm -rf $verify_dir
