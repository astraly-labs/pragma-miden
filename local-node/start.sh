#! Starts a fresh local Miden Node.
#! See: https://github.com/0xPolygonMiden/miden-node

rm -rf ./accounts ./data ./genesis.toml ./miden-node

miden-node store dump-genesis > genesis.toml
mkdir data
mkdir accounts
miden-node bundled bootstrap \
  --data-directory data \
  --accounts-directory accounts \
  --config genesis.toml 

miden-node bundled start \
  --data-directory data \
  --rpc.url http://0.0.0.0:57123