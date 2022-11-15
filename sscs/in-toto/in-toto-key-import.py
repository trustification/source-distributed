#!/usr/bin/python3

import sys
import json
from pathlib import Path
from securesystemslib.interface import import_ecdsa_publickey_from_file
from securesystemslib.keys import import_ecdsakey_from_private_pem

def convert(key_pem):
  private_key_pem = Path(key_pem).read_text();
  private_key = import_ecdsakey_from_private_pem(private_key_pem);
  print(f"private keyid: {private_key['keyid']}");
  f = open(f"{key_pem}.json", 'w');
  f.write(json.dumps(private_key));
  f.close();

  pub_json_dict = {};
  pub_json_dict['keytype'] = private_key['keytype'];
  pub_json_dict['scheme'] = private_key['scheme'];
  pub_json_dict['keyid_hash_algorithms'] = private_key['keyid_hash_algorithms'];
  pub_json_dict['keyval'] = { 'public': private_key['keyval']['public']};

  f = open(f"{key_pem}.pub.json", 'w');
  f.write(json.dumps(pub_json_dict));
  f.close();

  # Try importing the public key json generated previously to verify that the
  # keyid matches that of the private key.
  imported_pub = import_ecdsa_publickey_from_file(f"{key_pem}.pub.json");
  private_key_id = private_key['keyid'];
  public_key_id = imported_pub['keyid'];
  print(f"public keyid : {public_key_id}");
  if (private_key_id != public_key_id) :
      print('''Conversion from pem to json format failed
               If there has been a change in the order of fields that
               import_publickeys_from_file produces the id generation will
               not work''');
          

if __name__ == '__main__':
  convert(sys.argv[1]);
