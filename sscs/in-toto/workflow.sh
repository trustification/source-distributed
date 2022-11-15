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

# Create keys in securesystemslib json key format. This will generate two
# files: <keyname>.key.json and <keyname>.key..pub.json
echo "Import and convert cosign keys to securesystemslib json format"
./in-toto-key-import.py $PRIVATE_KEY

mkdir -p artifacts
echo "Create layout"
./create-layout.py $GITHUB_ORG $GITHUB_PROJECT ${PRIVATE_KEY}.json $PUBLIC_KEY

echo "Create steps"
./create-steps.sh $GITHUB_ORG $GITHUB_PROJECT ${PRIVATE_KEY}.json $PUBLIC_KEY

echo "Verify the artifacts"
pushd artifacts > /dev/null
in-toto-verify -v -t ecdsa --layout $GITHUB_PROJECT-layout.json --layout-keys=$PUBLIC_KEY
popd > /dev/null
