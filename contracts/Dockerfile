FROM rust:1.79-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Install Soroban CLI
RUN cargo install --locked soroban-cli --features opt

# Install cargo-nextest for faster testing
RUN cargo install cargo-nextest --locked

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy contract source
COPY contracts ./contracts
COPY tests ./tests

# Build contracts
RUN cargo build --release --target wasm32-unknown-unknown

# Optimize WASM binaries
RUN for contract in contracts/*/; do \
    contract_name=$(basename "$contract"); \
    soroban contract optimize \
        --wasm target/wasm32-unknown-unknown/release/${contract_name}.wasm; \
    done

CMD ["cargo", "nextest", "run"]
