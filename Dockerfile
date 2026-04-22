# Multi-stage build for PolyPulse
# This Dockerfile builds the Rust backend for deployment on Render.
# The frontend is served separately (static hosting).

# ── Stage 1: Build Rust backend ───────────────────────────────────────────────
FROM rust:latest AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files first for better layer caching
# (only re-runs cargo fetch when Cargo.toml/Cargo.lock change)
COPY backend/Cargo.toml backend/Cargo.lock ./

# Create a dummy main.rs so cargo can fetch deps without the full source
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo fetch

# Now copy the real source and build
COPY backend/src ./src
COPY backend/migrations ./migrations
COPY backend/.sqlx ./.sqlx

ENV SQLX_OFFLINE=true
RUN cargo build --release

# ── Stage 2: Final runtime image ──────────────────────────────────────────────
FROM debian:bookworm-slim AS backend

RUN apt-get update && apt-get install -y \
    libpq5 \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install sqlx-cli for running migrations
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install sqlx-cli --no-default-features --features postgres

WORKDIR /app

COPY --from=backend-builder /app/target/release/backend ./backend
COPY --from=backend-builder /app/migrations ./migrations

# Create startup script that runs migrations then starts the server
RUN echo '#!/bin/sh\nset -e\necho "Running database migrations..."\nsqlx migrate run\necho "Starting backend server..."\nexec ./backend' > /app/start.sh && \
    chmod +x /app/start.sh

EXPOSE 8000

CMD ["/app/start.sh"]
