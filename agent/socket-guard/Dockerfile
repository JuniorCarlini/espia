# Build from `agent/socket-guard/` (this crate is fully standalone, its
# own Cargo workspace — see Cargo.toml):
#   docker build -t espia-socket-guard .
# See docs/adr/0014-docker-container-stats.md.

FROM rust:1-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:stable-slim
COPY --from=builder /build/target/release/espia-socket-guard /usr/local/bin/espia-socket-guard
# Runs as root: it needs to reach /var/run/docker.sock, which is owned by
# root on the host. That's the only privilege this process uses — see
# main.rs, there's no code path here that issues a write call to the
# socket at all, let alone one reachable from an incoming request.
EXPOSE 2375
ENTRYPOINT ["espia-socket-guard"]
