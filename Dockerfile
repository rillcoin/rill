# Stage 1: Build
# rust:1.85-bookworm includes all build tooling on Debian Bookworm.
FROM rust:1.85-bookworm AS builder

WORKDIR /build

# Copy workspace manifests and lock file first to leverage layer caching.
# Dependencies are fetched and compiled before copying source, so that source
# changes do not invalidate the expensive dependency compilation layer.
COPY Cargo.toml Cargo.lock ./
COPY crates/rill-core/Cargo.toml crates/rill-core/Cargo.toml
COPY crates/rill-decay/Cargo.toml crates/rill-decay/Cargo.toml
COPY crates/rill-consensus/Cargo.toml crates/rill-consensus/Cargo.toml
COPY crates/rill-network/Cargo.toml crates/rill-network/Cargo.toml
COPY crates/rill-wallet/Cargo.toml crates/rill-wallet/Cargo.toml
COPY crates/rill-node/Cargo.toml crates/rill-node/Cargo.toml
COPY crates/rill-tests/Cargo.toml crates/rill-tests/Cargo.toml
COPY bins/rill-node/Cargo.toml bins/rill-node/Cargo.toml
COPY bins/rill-cli/Cargo.toml bins/rill-cli/Cargo.toml
COPY bins/rill-miner/Cargo.toml bins/rill-miner/Cargo.toml

# Create stub lib/main files so `cargo fetch` / dependency compilation succeeds
# without the real source. The real source is copied in the next step.
RUN find crates bins -name "Cargo.toml" | while read f; do \
      dir=$(dirname "$f"); \
      if grep -q '^\[\[bin\]\]' "$f" || grep -q '^name.*rill-node\|rill-cli\|rill-miner' "$f" 2>/dev/null; then \
        mkdir -p "$dir/src" && echo 'fn main() {}' > "$dir/src/main.rs"; \
      else \
        mkdir -p "$dir/src" && echo '' > "$dir/src/lib.rs"; \
      fi; \
    done

RUN cargo fetch --locked

# Now copy the full source tree and build for real.
COPY . .

RUN cargo build --release --locked \
    --bin rill-node \
    --bin rill-cli \
    --bin rill-miner

# Stage 2: Runtime
# debian:bookworm-slim gives a minimal, up-to-date Debian base without Rust.
FROM debian:bookworm-slim

# Install runtime dependencies only (ca-certificates for TLS, libssl for
# RocksDB's compression libraries if linked dynamically).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binaries from the builder stage.
COPY --from=builder /build/target/release/rill-node  /usr/local/bin/rill-node
COPY --from=builder /build/target/release/rill-cli   /usr/local/bin/rill-cli
COPY --from=builder /build/target/release/rill-miner /usr/local/bin/rill-miner

# P2P port and RPC port.
EXPOSE 18333 18332

# Blockchain data directory — mount a named volume here for persistence.
VOLUME /data

# Run as non-root for security.
RUN useradd --system --no-create-home --shell /usr/sbin/nologin rillnode
USER rillnode

ENTRYPOINT ["rill-node"]
# Default: listen on all interfaces, bind RPC to all interfaces so it is
# reachable from the host/other containers. Override via `command:` in Compose.
CMD ["--data-dir", "/data", \
     "--p2p-listen-addr", "0.0.0.0", \
     "--p2p-listen-port", "18333", \
     "--rpc-bind", "0.0.0.0", \
     "--rpc-port", "18332"]
