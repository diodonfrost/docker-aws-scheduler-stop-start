FROM rust:slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libc6-dev \
    pkg-config \
    perl \
    make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY scheduler/ .

RUN cargo build --release

FROM cgr.dev/chainguard/glibc-dynamic:latest

COPY --from=builder /build/target/release/scheduler /usr/local/bin/scheduler

ENTRYPOINT ["scheduler"]
