# fs-session — library crate, no binary
# This Containerfile is a placeholder for future CLI/daemon builds.
# Current use: cargo build --release (library only)
FROM docker.io/rust:1.83-slim AS builder

WORKDIR /build

COPY fs-libs/    fs-libs/
COPY fs-session/ fs-session/

WORKDIR /build/fs-session
RUN cargo build --release
