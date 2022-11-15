#!/bin/bash

if [ $# -ne 4 ]
  then
	echo "Usage: create-steps github_org github_project privatekey publickey"
	exit 1
fi

github_org=$1
project_name=$2
private_key_json=$3
public_key_json=$4
github_url=https://github.com/$github_org/${project_name}.git
workdir=work

## Create a work directory for all artifacts
rm -rf $workdir
mkdir $workdir
cp $private_key_json $public_key_json $workdir
pushd $workdir > /dev/null

echo "1) Cloning $github_url"
in-toto-run -n clone_project -k $private_key_json -t ecdsa --base-path $project_name --products Cargo.toml Cargo.lock examples README.md rustfmt.toml rust-toolchain.toml src tests -- git clone $github_url

echo "2) Run tests"
cargo test -q --manifest-path=${project_name}/Cargo.toml --no-run
in-toto-run -n run_tests -s -k $private_key_json -t ecdsa -- cargo test --manifest-path ${project_name}/Cargo.toml

echo "3) Copy artifacts"
cp *.link ../artifacts
cp $public_key_json ../artifacts

popd > /dev/null

rm -rf $workdir
