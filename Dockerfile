# Multi-stage build for PolyPulse

# ── Stage 1: Build Soroban contracts ─────────────────────────────────────────
FROM rust:1.79-slim AS soroban-builder

WORKDIR /soroban

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked soroban-cli --features opt

COPY contracts/Cargo.toml contracts/Cargo.lock contracts/rust-toolchain.toml ./
COPY contracts/contracts ./contracts
COPY contracts/tests ./tests

RUN cargo build --release --target wasm32-unknown-unknown

RUN for contract in contracts/*/; do \
    contract_name=$(basename "$contract"); \
    if [ -f "target/wasm32-unknown-unknown/release/${contract_name}.wasm" ]; then \
        soroban contract optimize \
            --wasm target/wasm32-unknown-unknown/release/${contract_name}.wasm; \
    fi; \
    done

# ── Stage 2: Build frontend ───────────────────────────────────────────────────
FROM node:20-alpine AS frontend-builder

WORKDIR /app

COPY frontend/package*.json ./
RUN npm ci

COPY frontend .

# Copy optimized contracts for frontend integration
COPY --from=soroban-builder /soroban/target/wasm32-unknown-unknown/release/*.optimized.wasm ./contracts/

RUN npm run build

# ── Stage 3: Build Rust backend ───────────────────────────────────────────────
FROM rust:1.79-slim AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations

RUN cargo build --release

# ── Stage 4: Final runtime image (backend) ────────────────────────────────────
FROM debian:bookworm-slim AS backend

RUN apt-get update && apt-get install -y \
    libpq5 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/target/release/backend ./backend
COPY --from=backend-builder /app/migrations ./migrations

EXPOSE 8000

CMD ["./backend"]
