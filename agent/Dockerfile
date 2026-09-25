# Build from the `agent/` directory (Docker context = this monorepo's
# Cargo workspace, so the headless crate can see its `../core` sibling):
#   docker build -t espia-headless .
# See docs/adr/0013-headless-agent.md and docs/adr/0014-docker-container-stats.md.

FROM rust:1-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p espia-headless

FROM debian:stable-slim
COPY --from=builder /build/target/release/espia-headless /usr/local/bin/espia-headless
# Runs as root, not a dedicated user: this image is meant to be run with
# /var/run/docker.sock mounted in (for GET /containers), and anyone who can
# reach that socket already has root-equivalent control of the whole Docker
# host — a non-root USER here would be a false sense of extra isolation,
# not real defense in depth. See docs/adr/0014-docker-container-stats.md.
EXPOSE 8080
ENTRYPOINT ["espia-headless"]
