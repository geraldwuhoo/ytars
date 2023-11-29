# chef
FROM docker.io/library/rust:1.74.0 AS chef
RUN rustup target add x86_64-unknown-linux-musl && \
    apt update && \
    apt install -y musl-tools musl-dev && \
    rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /usr/src

# planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder
FROM chef AS builder
COPY --from=planner /usr/src/recipe.json recipe.json

RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin ytars

# Clean image
FROM scratch
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/ytars /usr/bin/ytars
ENTRYPOINT ["ytars"]
