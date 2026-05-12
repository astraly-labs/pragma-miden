#!/bin/sh
# Initialize oracle workspace from injected secrets and embedded config
set -e

WORKSPACE="${ORACLE_WORKSPACE_PATH:-/data/oracle-workspace}"

mkdir -p "${WORKSPACE}/keystore"
mkdir -p "${WORKSPACE}/miden_storage"

# Prefer a config mounted from a K8s secret at /secrets/config/pragma_miden.json
# (used by deployments that point this image at a custom oracle), otherwise
# fall back to the one baked into the image.
if [ -f /secrets/config/pragma_miden.json ]; then
  cp /secrets/config/pragma_miden.json "${WORKSPACE}/pragma_miden.json"
  echo "Using pragma_miden.json from mounted secret"
else
  cp /app/pragma_miden.json "${WORKSPACE}/pragma_miden.json"
  echo "Using pragma_miden.json baked into image"
fi

# Copy keystore files from mounted secrets
for f in /secrets/keystore/*; do
  [ -e "$f" ] || continue
  filename=$(basename "$f")
  cp "$f" "${WORKSPACE}/keystore/${filename}"
  chmod 600 "${WORKSPACE}/keystore/${filename}"
done

echo "Workspace initialized at ${WORKSPACE}"
ls -la "${WORKSPACE}/keystore/" 2>/dev/null || true
