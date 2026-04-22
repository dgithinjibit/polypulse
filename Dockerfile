# Multi-stage build for PolyPulse

# Stage 1: Build Soroban contracts
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

# Stage 2: Rust Backend
FROM rust:1.79-slim AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS backend

RUN apt-get update && apt-get install -y \
    libpq5 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/target/release/backend ./backend

EXPOSE 8000

CMD ["./backend"]

# Stage 3: Frontend
FROM node:20-alpine AS frontend

WORKDIR /app

COPY frontend/package*.json ./
RUN npm ci --only=production

COPY frontend .

# Copy optimized contracts for frontend integration
COPY --from=soroban-builder /soroban/target/wasm32-unknown-unknown/release/*.optimized.wasm ./contracts/

RUN npm run build

EXPOSE 5173

CMD ["npm", "run", "preview", "--", "--host"]
