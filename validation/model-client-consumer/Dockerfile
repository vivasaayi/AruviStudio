# Verifies a clean consumer can vendor the pinned Git dependency and then build
# without network access. Consumers should commit their own Cargo.lock.
FROM rust:1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS test
WORKDIR /consumer
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN mkdir -p .cargo && cargo vendor --locked vendor > .cargo/config.toml
RUN --network=none cargo test --locked --offline

FROM test AS probe
RUN --network=none cargo build --locked --offline --bin host_probe
FROM scratch AS probe-artifact
COPY --from=probe /consumer/target/debug/host_probe /host_probe
