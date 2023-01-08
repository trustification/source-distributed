#!/bin/bash
## This script is intended to be run manually and simulate the process of
# creating in-toto steps and layout.
#
# Output will be produced in the artifacts directory which includes the link
# files, the layout, and the public key.
GITHUB_PROJECT="${2:-source-distributed}"
GITHUB_TOKEN=$3
PRIVATE_KEY=cosign.key
PUBLIC_KEY=${PRIVATE_KEY}.pub.json

BRANCH=$(git branch --show-current)

echo "workflow.sh arguments: $GITHUB_PROJECT $GITHUB_TOKEN"

pushd ../../ > /dev/null

## Generate in-toto layout and link files
if [ -z $GITHUB_TOKEN ]; then
	echo "Generating without token...."
	cargo r --bin cargo-in-toto-sign
else
	echo "Generating with token...."
	cargo r --bin cargo-in-toto-sign --provider-token=$GITHUB_TOKEN
fi

cargo r --bin cargo-in-toto-verify -- -d $GITHUB_PROJECT -a sscs/in-toto/artifacts/$BRANCH -p $PWD

popd > /dev/null
