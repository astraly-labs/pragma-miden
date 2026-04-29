#!/bin/bash
# Starts a fresh local Miden Node (v0.14).
# See: https://docs.miden.xyz/next/builder/tutorials/miden_node_setup

set -e

rm -rf ./accounts ./data

mkdir data
mkdir accounts
miden-node bundled bootstrap \
  --data-directory data \
  --accounts-directory accounts

miden-node bundled start \
  --data-directory data \
  --rpc.url http://0.0.0.0:57291