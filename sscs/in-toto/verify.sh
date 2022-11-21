echo "Verify the artifacts.tar with cosign"
commit_id=$1
github_project=$2
cosign verify-blob -d --bundle=artifacts/${commit_id}.bundle artifacts/${commit_id}.tar

mkdir -p working
tar xvf artifacts/${commit_id}.tar --directory working
pushd working
in-toto-verify -v -t ecdsa --layout $github_project-layout.json --layout-keys=cosign.key.pub.json
popd
rm -rf working

