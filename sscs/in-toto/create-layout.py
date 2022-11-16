#!/usr/bin/python3

import sys
import json

from securesystemslib.interface import import_publickeys_from_file
from in_toto.models.layout import Layout
from in_toto.models.metadata import Metablock

def process(github_org, github_project, private_key_file, public_key_file):
  print(f"Processing https://github.com/{github_org}/{github_project}.git")

  private_key_str = open(private_key_file, 'r').read();
  private_key = eval(private_key_str);
  print(private_key['keyid']);

  public_key_pem = private_key["keyval"]["public"];
  dict = import_publickeys_from_file([public_key_file], ["ecdsa"]);
  print(dict[private_key['keyid']]['keyid']);
  public_key = dict[private_key['keyid']];
  print(public_key['keyid']);

  layout = Layout.read({
      "_type": "layout",
      "keys": {
          public_key["keyid"]: public_key,
      },
      "steps": [{
          "name": "clone_project",
          "expected_materials": [],
          "expected_products": [
              ["CREATE", github_project],
              ["ALLOW", f"{github_project}/*"],
              ["ALLOW", f"{github_project}-layout.json"],
          ],
          "pubkeys": [public_key["keyid"]],
          "expected_command": [
              "git",
              "clone",
              f"https://github.com/{github_org}/{github_project}.git"
          ],
          "threshold": 1,
        },{
          "name": "run_tests",
          "expected_materials": [
              ["MATCH", f"{github_project}/*", "WITH", "PRODUCTS", "FROM", "clone_project"],
              ["ALLOW", "Cargo.toml"],
              ["DISALLOW", "*"],
          ],
          "expected_products": [
              ["ALLOW", "Cargo.lock"],
              ["ALLOW", "cosign.key.json"],
              ["ALLOW", "cosign.key.pub.json"],
              ["DISALLOW", "*"]],
          "pubkeys": [public_key["keyid"]],
          "expected_command": [
              "cargo",
              "test",
              "--manifest-path",
              f"{github_project}/Cargo.toml",
              ],
          "threshold": 1,
        }],
      "inspect": [{
          "name": "cargo-fetch",
          "expected_materials": [
              ["MATCH", f"{github_project}/*", "WITH", "PRODUCTS", "FROM", "clone_project"],
              ["ALLOW", f"{github_project}/target"],
              ["ALLOW", "cosign.key.json"],
              ["ALLOW", "cosign.key.pub.json"],
              ["ALLOW", f"{github_project}-layout.json"],
              ["DISALLOW", "*"],
          ],
          "expected_products": [
              ["MATCH", f"{github_project}/Cargo.toml", "WITH", "PRODUCTS", "FROM", "clone_project"],
              ["MATCH", "*", "WITH", "PRODUCTS", "FROM", "clone_project"],
              ["ALLOW", f"{github_project}/target"],
              ["ALLOW", public_key_file],
              ["ALLOW", "cosign.key.json"],
              ["ALLOW", "cosign.key.pub.json"],
              ["ALLOW", f"{github_project}-layout.json"],
          ],
          "run": [
              "git",
              "clone",
              f"git@github.com:{github_org}/{github_project}.git"
          ],
        }],
  })

  metadata = Metablock(signed=layout)

  print(f"Creating artifacts/{github_project}-layout.json file")
  print(metadata);
  metadata.sign(private_key)
  metadata.dump(f"artifacts/{github_project}-layout.json")

if __name__ == '__main__':
  process(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])
