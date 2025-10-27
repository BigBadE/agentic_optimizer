# Minimal Debian container for Merlin CI with Rust toolchain and coverage tools
FROM debian:bookworm-slim

# Install all system dependencies including mold for faster linking and lcov for coverage
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
    lcov \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Install rustup with nightly toolchain (version specified later via rust-toolchain.toml)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal

# Install cargo-binstall first, then use it to install other tools from pre-built binaries
RUN cargo install --locked cargo-binstall && \
    cargo binstall --no-confirm \
    cargo-nextest \
    cargo-llvm-cov \
    cargo-sweep \
    grcov \
    gungraun-runner && \
    cargo uninstall cargo-binstall && \
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git ~/.cargo/registry ~/.cargo/git

# Create a non-root user for running builds
RUN useradd -m -s /bin/bash builder && \
    mkdir -p /workspace && \
    chown -R builder:builder /workspace && \
    chown -R builder:builder /usr/local/rustup /usr/local/cargo

USER builder
WORKDIR /workspace

# Pre-build workspace dependencies by copying entire workspace and building
# This layer will be cached as long as dependencies don't change
COPY --chown=builder:builder . .

# Install the exact toolchain from rust-toolchain.toml and add cranelift + llvm-tools
RUN rustup show && \
    rustup component add rustc-codegen-cranelift-preview llvm-tools-preview

# Copy cargo config to builder's home directory for optimized builds
RUN mkdir -p /home/builder/.cargo && \
    cp -r .cargo/* /home/builder/.cargo/

# Build all workspace dependencies (release and test)
# The compiled dependencies will be cached in the target directory
RUN cargo build --release --workspace && \
    cargo build --tests --workspace

# Remove all workspace source files, keeping only the compiled dependencies and .cargo config
# This ensures fresh builds in CI while reusing dependency compilation
RUN find . -mindepth 1 -maxdepth 1 ! -name 'target' ! -name '.cargo' -exec rm -rf {} + && \
    find target -type f \( -name '*merlin*' -o -name '*quality*' -o -name '*criterion*' -o -name '*gungraun*' \) \
    \( -name '*.rlib' -o -name '*.so' -o -name '*.a' -o -perm -111 \) -delete

# The actual workspace source code will be provided by the CI workflow
# All external dependencies are now pre-compiled and cached in /workspace/target