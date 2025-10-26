# Minimal Debian container for Merlin CI with Rust toolchain and coverage tools
FROM debian:bookworm-slim

# Install minimal dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    build-essential \
    valgrind \
    python3 \
    jq \
    lcov \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain from rust-toolchain.toml specification
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Install rustup and the specified nightly toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal && \
    rustup toolchain install nightly-2025-10-26 --profile minimal --component rustfmt,clippy,rust-src,llvm-tools-preview && \
    rustup default nightly-2025-10-26

# Install cargo tools
RUN cargo install --locked cargo-nextest cargo-llvm-cov cargo-sweep cargo-binstall grcov

# Create a non-root user for running builds
RUN useradd -m -s /bin/bash builder && \
    mkdir -p /workspace && \
    chown -R builder:builder /workspace

USER builder
WORKDIR /workspace

# Set environment variables for optimal build performance
ENV CARGO_INCREMENTAL=0 \
    CARGO_TERM_COLOR=always \
    RUST_BACKTRACE=1
