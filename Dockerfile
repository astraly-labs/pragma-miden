# ── Stage 1: Build Rust binaries ─────────────────────────────────────────────
FROM --platform=linux/amd64 rust:1.82-slim AS rust-builder

RUN apt-get update && apt-get install -y \
  pkg-config libssl-dev libsqlite3-dev clang \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY examples/ ./examples/

RUN cargo build --release -p pm-oracle-cli

# ── Stage 2: Build Next.js app ────────────────────────────────────────────────
FROM --platform=linux/amd64 node:20-slim AS next-builder

RUN npm install -g pnpm

WORKDIR /app/oracle-explorer

COPY oracle-explorer/package.json oracle-explorer/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY oracle-explorer/ ./

ENV NEXT_TELEMETRY_DISABLED=1
RUN pnpm build

# ── Stage 3: Runtime ──────────────────────────────────────────────────────────
FROM --platform=linux/amd64 node:20-slim AS runner

RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Next.js standalone build
COPY --from=next-builder /app/oracle-explorer/.next/standalone ./
COPY --from=next-builder /app/oracle-explorer/.next/static ./.next/static

# Rust binary
COPY --from=rust-builder /app/target/release/pm-oracle-cli /usr/local/bin/pm-oracle-cli

# Workspace config (non-secret)
COPY pragma_miden.json ./pragma_miden.json

# Init script
COPY entrypoint-init.sh ./entrypoint-init.sh
RUN chmod +x ./entrypoint-init.sh

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV NETWORK=testnet
ENV ORACLE_WORKSPACE_PATH=/data/oracle-workspace
ENV CLI_PATH=/usr/local/bin

EXPOSE 3000

CMD ["node", "server.js"]
