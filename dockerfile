FROM python@sha256:9ed09f78253eb4f029f3d99e07c064f138a6f1394932c3807b3d0738a674d33b AS base
# python:3.13.4-slim

# Rust
RUN apt-get update && apt-get install -y curl build-essential pkg-config libssl-dev && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    . "$HOME/.cargo/env"

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

COPY rust_bot ./rust_bot
COPY python_llm ./python_llm
COPY Cargo.toml Cargo.lock ./

RUN cd rust_bot && cargo build --release

FROM python@sha256:9ed09f78253eb4f029f3d99e07c064f138a6f1394932c3807b3d0738a674d33b

# Copying from base image
WORKDIR /app
COPY --from=base /app/python_llm ./python_llm
COPY --from=base /app/target/release/rust_bot ./bot
COPY .env /app/.env

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential gcc cmake &&\
    apt-get clean &&\
    rm -rf /var/lib/apt/lists/*

RUN pip install -r /app/python_llm/requirements.txt

CMD ["./bot"]