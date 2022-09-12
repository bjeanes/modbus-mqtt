# -*- mode: dockerfile -*-

FROM rust:1.63 AS builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt-get --no-install-recommends install -y \
    libudev-dev

# Build a cacheable layer with project dependencies
RUN USER=rust cargo new /home/rust/sungrow-winets
RUN USER=rust cargo new /home/rust/tokio_modbus-winets
RUN USER=rust cargo new /home/rust/modbus-mqtt
WORKDIR /home/rust/modbus-mqtt
ADD --chown=rust:rust Cargo.lock modbus-mqtt/Cargo.toml ./
RUN mkdir -p /home/rust/modbus-mqtt/target/release
RUN --mount=type=cache,target=/home/rust/modbus-mqtt/target,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release

# # Delete files & directories which shouldn't exist for the workspace
# RUN rm -rf src

# Add our source code.
ADD --chown=rust:rust . ./

# Build our application.
RUN --mount=type=cache,target=/home/rust/modbus-mqtt/target,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release && mv target/release/modbus-mqtt ./bin

# Now, we need to build our _real_ Docker container, copying in `bump-api`.
FROM debian:bullseye-slim

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt-get --no-install-recommends install -y \
    libudev1

COPY --from=builder \
    /home/rust/modbus-mqtt/bin \
    /usr/local/bin/modbus-mqtt

ENV RUST_LOG=warn,modbus_mqtt=info

ENTRYPOINT ["/usr/local/bin/modbus-mqtt"]
