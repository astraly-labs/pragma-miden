#! Starts a fresh local Miden Node.
#! See: https://github.com/0xPolygonMiden/miden-node

rm -rf ./accounts ./blocks ./genesis.dat ./genesis.toml ./miden-node.toml ./miden-store.sqlite3 ./miden-store.sqlite3-shm ./miden-store.sqlite3-wal

miden-node init --config-path  miden-node.toml --genesis-path genesis.toml
miden-node make-genesis --inputs-path genesis.toml --output-path genesis.dat --force
miden-node start --config miden-node.toml node
