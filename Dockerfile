ARG FINAL_IMAGE=scratch-final

# chef
FROM docker.io/library/rust:1.91.1-bookworm AS chef
RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && \
    apt-get install -y --no-install-recommends musl-dev=1.2.3-1 && \
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
ARG CI_COMMIT_TAG
ARG CI_COMMIT_SHORT_SHA
RUN cargo build --release --target x86_64-unknown-linux-musl --bin ytars

# Build Python dependencies
FROM docker.io/library/python:3.14.0-slim-bookworm AS python-builder
WORKDIR /usr/src
COPY requirements.txt requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Deno dependency for yt-dlp
FROM docker.io/denoland/deno:bin-2.5.6 AS deno

# Distroless image to run Python
FROM gcr.io/distroless/python3-debian12@sha256:afdfc6df9eb96cf65576d4d59af48c4b5dac29ad62066aee879f12b303298aac AS python-final
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/ytars /usr/bin/ytars
COPY --from=builder /usr/lib/ssl/ /usr/local/ssl/
COPY --from=builder /etc/ssl/ /etc/ssl/
COPY --from=python-builder /usr/local/lib/python3.13/site-packages /usr/local/lib/python3.13/site-packages
COPY --from=deno /deno /usr/bin/deno
COPY download.py /usr/bin/download.py
ENV PYTHONPATH /usr/local/lib/python3.13/site-packages
ENV ENABLE_YT_DLP true
ENV YT_DLP_SCRIPT_PATH /usr/bin/download.py

# Clean image
FROM scratch AS scratch-final
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/ytars /usr/bin/ytars
COPY --from=builder /usr/lib/ssl/ /usr/local/ssl/
COPY --from=builder /etc/ssl/ /etc/ssl/

FROM ${FINAL_IMAGE}

ENTRYPOINT [ "ytars" ]
