# Minimal Debian container for Merlin CI with Rust toolchain and coverage tools
FROM debian:bookworm-slim

# Install all system dependencies including mold for faster linking
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    build-essential \
    valgrind \
    python3 \
    python3-pip \
    jq \
    mold \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Install rustup with nightly toolchain (version specified later via rust-toolchain.toml)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal

# Install cargo tools (including cargo-binstall for gungraun-runner)
RUN cargo install --locked cargo-nextest cargo-llvm-cov cargo-sweep cargo-binstall grcov && \
    cargo binstall --no-confirm gungraun-runner --version 0.17 && \
    cargo uninstall cargo-binstall && \
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git ~/.cargo/registry ~/.cargo/git

# Create a non-root user for running builds
RUN useradd -m -s /bin/bash builder && \
    mkdir -p /workspace && \
    chown -R builder:builder /workspace

USER builder
WORKDIR /workspace

# Pre-build workspace dependencies by copying entire workspace and building
# This layer will be cached as long as dependencies don't change
COPY --chown=builder:builder . .

# Install the exact toolchain from rust-toolchain.toml and add cranelift
RUN rustup show && \
    rustup component add rustc-codegen-cranelift-preview

# Copy cargo config to builder's home directory for optimized builds
RUN mkdir -p /home/builder/.cargo && \
    cp -r .cargo/* /home/builder/.cargo/

# Build all workspace dependencies (release and test)
# The compiled dependencies will be cached in the target directory
RUN cargo build --release --workspace && \
    cargo build --tests --workspace

# Remove all workspace source files, keeping only the compiled dependencies
# This ensures fresh builds in CI while reusing dependency compilation
RUN find . -mindepth 1 -maxdepth 1 ! -name 'target' -exec rm -rf {} + && \
    find target -type f \( -name '*merlin*' -o -name '*quality*' -o -name '*criterion*' -o -name '*gungraun*' \) \
    \( -name '*.rlib' -o -name '*.so' -o -name '*.a' -o -perm -111 \) -delete

# The actual workspace source code will be provided by the CI workflow
# All external dependencies are now pre-compiled and cached in /workspace/target