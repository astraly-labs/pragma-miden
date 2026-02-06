#!/bin/bash
# Starts a fresh local Miden Node.
# See: https://docs.miden.xyz/miden-tutorials/miden_node_setup

rm -rf ./accounts ./data

mkdir data
mkdir accounts
miden-node bundled bootstrap \
  --data-directory data \
  --accounts-directory accounts

miden-node bundled start \
  --data-directory data \
  --rpc.url http://0.0.0.0:57291