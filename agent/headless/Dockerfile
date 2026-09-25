# Build from the `agent/` directory:
#   docker build -t espia-headless -f headless/Dockerfile .
# See docs/adr/0013-headless-agent.md.

FROM rust:1-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p espia-headless

FROM debian:stable-slim
RUN useradd --system --no-create-home espia
COPY --from=builder /build/target/release/espia-headless /usr/local/bin/espia-headless
USER espia
EXPOSE 8080
ENTRYPOINT ["espia-headless"]
