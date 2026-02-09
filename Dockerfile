ARG FINAL_IMAGE=scratch-final

# chef
FROM docker.io/library/rust:1.92.0-bookworm AS chef
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
FROM docker.io/library/python:3.14.2-slim-bookworm AS python-builder
WORKDIR /usr/src
COPY requirements.txt requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Deno dependency for yt-dlp
FROM docker.io/denoland/deno:bin-2.5.6 AS deno

# ffmpeg dependency for yt-dlp
FROM docker.io/library/alpine:3.23.3 AS ffmpeg
WORKDIR /
SHELL [ "/bin/ash", "-o", "pipefail", "-c" ]
RUN wget -q "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-linux-x64.gz" -O- | gunzip - > ffmpeg && \
    chmod +x ./ffmpeg

# Distroless image to run Python
FROM gcr.io/distroless/python3-debian12@sha256:8ce6bba3f793ba7d834467dfe18983c42f9b223604970273e9e3a22b1891fc27 AS python-final
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/ytars /usr/bin/ytars
COPY --from=builder /usr/lib/ssl/ /usr/local/ssl/
COPY --from=builder /etc/ssl/ /etc/ssl/
COPY --from=python-builder /usr/local/lib/python3.14/site-packages /usr/local/lib/python3.14/site-packages
COPY --from=deno /deno /usr/bin/deno
COPY --from=ffmpeg /ffmpeg /usr/bin/ffmpeg
COPY download.py /usr/bin/download.py
ENV PYTHONPATH /usr/local/lib/python3.14/site-packages
ENV ENABLE_YT_DLP true
ENV YT_DLP_SCRIPT_PATH /usr/bin/download.py

# Clean image
FROM scratch AS scratch-final
COPY --from=builder /usr/src/target/x86_64-unknown-linux-musl/release/ytars /usr/bin/ytars
COPY --from=builder /usr/lib/ssl/ /usr/local/ssl/
COPY --from=builder /etc/ssl/ /etc/ssl/

FROM ${FINAL_IMAGE}

ENTRYPOINT [ "ytars" ]
