#!/bin/sh
# Initialize oracle workspace from injected secrets and embedded config
set -e

WORKSPACE="${ORACLE_WORKSPACE_PATH:-/data/oracle-workspace}"

mkdir -p "${WORKSPACE}/keystore"
mkdir -p "${WORKSPACE}/miden_storage"

# Copy embedded pragma_miden.json
cp /app/pragma_miden.json "${WORKSPACE}/pragma_miden.json"

# Copy keystore files from mounted secrets
for f in /secrets/keystore/*; do
  [ -e "$f" ] || continue
  filename=$(basename "$f")
  cp "$f" "${WORKSPACE}/keystore/${filename}"
  chmod 600 "${WORKSPACE}/keystore/${filename}"
done

echo "Workspace initialized at ${WORKSPACE}"
ls -la "${WORKSPACE}/keystore/" 2>/dev/null || true
