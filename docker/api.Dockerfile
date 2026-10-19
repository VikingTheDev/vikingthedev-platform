# syntax=docker/dockerfile:1
# docker build -f docker/api.Dockerfile --build-arg GIT_SHA=<sha> [--build-arg FEATURES=e2e] .

# The tag pins the same toolchain as rust-toolchain.toml, which isn't copied: its components aren't needed to build.
FROM rust:1.99.1-slim-trixie@sha256:4df2ae8cfdfebdfc13a65da7f253b73d68983dc0339f2719bc17a279c913b13d AS chef
RUN cargo install cargo-chef --locked --version 0.1.78
WORKDIR /build

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Only the CI-stack image sets FEATURES=e2e, and it's never pushed (ADR 0002 point 3).
ARG FEATURES=""
COPY --from=planner /build/recipe.json ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo chef cook --release --locked --package api --features "${FEATURES}" --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
ARG GIT_SHA=dev
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked --package api --features "${FEATURES}"

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:e792ab3d241a468a4fd7519ddbbebe66b49b5f365771716ea688ad40b6c6f1c2
COPY --from=builder /build/target/release/api /api
EXPOSE 8080 8081
ENTRYPOINT ["/api"]
