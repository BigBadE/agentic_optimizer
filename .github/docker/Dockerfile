# Minimal Debian container for Merlin CI with Rust toolchain and coverage tools
FROM debian:bookworm-slim

# Install minimal dependencies including mold for faster linking
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    build-essential \
    valgrind \
    python3 \
    jq \
    mold \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain from rust-toolchain.toml specification
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Install rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal

# Copy toolchain configuration and install specified toolchain with cranelift
COPY rust-toolchain.toml /tmp/rust-toolchain.toml
RUN cd /tmp && \
    rustup show && \
    rustup component add rustc-codegen-cranelift-preview && \
    rm /tmp/rust-toolchain.toml

# Install cargo tools
RUN cargo install --locked cargo-nextest cargo-llvm-cov cargo-sweep cargo-binstall grcov

# Install gungraun-runner for benchmarking (version must match workspace dependency)
RUN cargo binstall --no-confirm gungraun-runner --version 0.17

# Remove binstall now
RUN cargo uninstall --bin binstall

# Install Python 3 and pip for benchmark parsing scripts
RUN apt-get update && \
    apt-get install -y --no-install-recommends python3-pip && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user for running builds
RUN useradd -m -s /bin/bash builder && \
    mkdir -p /workspace && \
    chown -R builder:builder /workspace

# Copy cargo config for optimized builds
COPY .cargo /usr/local/cargo/config-template
RUN mkdir -p /home/builder/.cargo && \
    cp -r /usr/local/cargo/config-template/* /home/builder/.cargo/ && \
    chown -R builder:builder /home/builder/.cargo

USER builder
WORKDIR /workspace

# Pre-build workspace dependencies by copying entire workspace and building
# This layer will be cached as long as dependencies don't change
COPY --chown=builder:builder . .

# Build all workspace dependencies (release and test)
# The compiled dependencies will be cached in the target directory
RUN cargo build --release --workspace && \
    cargo build --tests --workspace

# Remove all workspace source files, keeping only the compiled dependencies
# This ensures fresh builds in CI while reusing dependency compilation
RUN find . -mindepth 1 -maxdepth 1 ! -name 'target' ! -name '.cargo' -exec rm -rf {} + && \
    find target -name '*merlin*' -o -name '*quality*' -o -name '*criterion*' -o -name '*gungraun*' | \
    grep -E '(\.rlib|\.so|\.a|/merlin|/quality|/criterion|/gungraun)' | xargs rm -f