# -*- mode: dockerfile -*-

FROM rust:1.63-alpine AS builder

RUN apk add --no-cache musl-dev

# Build a cacheable layer with project dependencies
RUN USER=rust cargo new /home/rust/sungrow-winets
RUN USER=rust cargo new /home/rust/tokio_modbus-winets
RUN USER=rust cargo new /home/rust/modbus-mqtt
WORKDIR /home/rust/modbus-mqtt
ADD --chown=rust:rust Cargo.lock modbus-mqtt/Cargo.toml ./
RUN cargo build --release

# # Delete files & directories which shouldn't exist for the workspace
# RUN rm -rf src

# Add our source code.
ADD --chown=rust:rust . ./

# Build our application.
RUN cargo build --release

# Now, we need to build our _real_ Docker container, copying in `bump-api`.
FROM alpine:latest
RUN apk --no-cache add ca-certificates

COPY --from=builder \
    /home/rust/modbus-mqtt/target/release/modbus-mqtt \
    /usr/local/bin/

ENV RUST_LOG=warn,modbus_mqtt=info

ENTRYPOINT ["/usr/local/bin/modbus-mqtt"]
