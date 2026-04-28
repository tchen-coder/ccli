FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && cargo build --release --target aarch64-unknown-linux-musl \
    && rm -rf src

COPY src/ src/
RUN touch src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && cargo build --release --target aarch64-unknown-linux-musl

FROM scratch AS output
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/ccli /ccli-linux-amd64
COPY --from=builder /src/target/aarch64-unknown-linux-musl/release/ccli /ccli-linux-arm64
