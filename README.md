# PolyPulse — Stellar Prediction Markets

A Web3 prediction markets dApp where users trade on real-world outcomes using Stellar wallets. Built for the Stellar hackathon, deployed at [polypulse.co.ke](https://polypulse.co.ke).

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 18 + TypeScript + Vite + Tailwind CSS |
| Backend | Rust + Axum + SQLx |
| Realtime | WebSockets via Axum + Redis Pub/Sub |
| Blockchain | Stellar (Soroban Smart Contracts) |
| Database | PostgreSQL |
| Cache | Redis |
| Auth | JWT + Freighter Wallet signature verification |

---

## Quick Start (Development)

### Prerequisites
- Rust 1.79+
- Node.js 18+
- PostgreSQL 15+
- Redis 7+
- Stellar CLI (for contract deployment)

### Backend

```bash
cd backend
cargo build

# Copy and configure environment
cp ../.env.example ../.env
# Edit .env — set JWT_SECRET, DATABASE_URL, REDIS_URL

# Run migrations
sqlx migrate run

# Start server
cargo run
```

### Frontend

```bash
cd frontend
npm install
cp .env.development .env.local   # adjust if needed
npm run dev
```

App runs at `http://localhost:5173`, API at `http://localhost:8000`.

---

## Environment Variables

### Backend (`.env` in root)

| Variable | Required | Description |
|---|---|---|
| `JWT_SECRET` | ✅ | JWT signing secret — generate with `openssl rand -base64 32` |
| `PORT` | ✅ | Server port (default: 8000) |
| `DATABASE_URL` | ✅ | PostgreSQL URL, e.g. `postgres://user:pass@localhost:5432/polypulse` |
| `REDIS_URL` | ✅ | Redis URL for WebSocket pub/sub, e.g. `redis://localhost:6379` |
| `CORS_ALLOWED_ORIGINS` | ✅ | Frontend origin(s), e.g. `https://polypulse.co.ke` |
| `STELLAR_NETWORK` | ✅ | `mainnet` or `testnet` |
| `STELLAR_RPC_URL` | ✅ | Soroban RPC URL |
| `STELLAR_HORIZON_URL` | ✅ | Horizon API URL |
| `STELLAR_MARKET_CONTRACT_ID` | — | Deployed market contract address |
| `STELLAR_CHALLENGE_CONTRACT_ID` | — | Deployed challenge contract address |

### Frontend (`frontend/.env.local`)

| Variable | Description |
|---|---|
| `VITE_API_URL` | Rust backend URL, e.g. `https://api.polypulse.co.ke` |
| `VITE_API_HOST` | Backend host for WebSocket URL construction |
| `VITE_WS_URL` | WebSocket base URL, e.g. `wss://api.polypulse.co.ke` |
| `VITE_STELLAR_NETWORK` | `mainnet` or `testnet` |
| `VITE_STELLAR_HORIZON_URL` | Horizon API URL |
| `VITE_STELLAR_MARKET_CONTRACT_ID` | Market contract address |
| `VITE_STELLAR_CHALLENGE_CONTRACT_ID` | Challenge contract address |

---

## Production Deployment

### 1. Server Setup

```bash
# Install system dependencies
sudo apt update && sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    nodejs \
    npm \
    redis-server \
    postgresql

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repo
git clone https://github.com/your-org/polypulse.git
cd polypulse
```

### 2. Backend

```bash
cd backend

# Configure environment
cp ../.env.example ../.env
nano ../.env   # Set JWT_SECRET, DATABASE_URL, REDIS_URL, CORS_ALLOWED_ORIGINS

# Build release
cargo build --release

# Run migrations
sqlx migrate run

# Run server
./target/release/backend
```

### 3. Frontend

```bash
cd frontend
npm install
cp .env.production .env.local   # adjust VITE_API_URL if needed
npm run build
# Serve dist/ with nginx or any static host
```

### 4. Nginx (example)

```nginx
server {
    listen 443 ssl;
    server_name polypulse.co.ke;

    # Frontend static files
    root /var/www/polypulse/frontend/dist;
    index index.html;
    try_files $uri $uri/ /index.html;

    # API proxy
    location /api/ {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # WebSocket proxy
    location /ws {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

### 5. Docker Compose

```bash
# Create .env file with required variables
cp .env.example .env
nano .env   # Set POSTGRES_PASSWORD, JWT_SECRET, etc.

docker compose up -d
```

---

## Stellar Smart Contracts

### Soroban Market Contract (`contracts/contracts/market/`)
Stellar Soroban contract implementing LMSR prediction markets.

**Features:**
- Create prediction markets with multiple options
- Buy/sell shares using LMSR pricing algorithm
- Resolve markets and distribute payouts
- XLM token integration for payments

**Deployment:**
```bash
cd contracts
stellar contract build
stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/market.wasm \
    --network testnet
```

### Soroban Challenge Contract (`contracts/contracts/challenge/`)
Direct wager contract for 1v1 challenges.

**Features:**
- Create open or direct challenges
- Accept challenges with stake matching
- Resolve challenges with winner determination
- Automatic payout distribution

**Deployment:**
```bash
cd contracts
stellar contract build
stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/challenge.wasm \
    --network testnet
```

---

## Authentication

All authentication is wallet-based using Freighter (Stellar wallet):

1. **Frontend** requests a nonce from `/api/v1/auth/nonce`
2. **User** signs a message containing the nonce with their Freighter wallet
3. **Frontend** sends `public_key + signature + message` to `/api/v1/auth/login`
4. **Backend** verifies the ed25519 signature, marks nonce as used
5. **Backend** issues JWT access + refresh tokens
6. **Frontend** stores tokens in `localStorage` and attaches `Authorization: Bearer <token>` to all API requests
7. **WebSocket** connections pass `?token=<jwt>` in the URL query string

---

## API Rate Limits

| Endpoint type | Limit |
|---|---|
| Anonymous | 60 req/min |
| Authenticated | 300 req/min |
| Auth (login/nonce) | 10 req/min |
| Trading (bet/sell) | 30 req/min |

---

## Security Notes

- JWT access tokens expire in **30 minutes**; refresh tokens in **7 days** with rotation
- WebSocket connections require a valid JWT — unauthenticated connections are rejected
- CORS is restricted to explicit origins — wildcard (`*`) is never allowed
- All security headers (HSTS, XSS protection, CSP, X-Frame-Options) are enabled in production
- Wallet nonces are single-use and expire after **5 minutes**
- Rate limiting is applied at the API layer on all sensitive endpoints

---

## Project Structure

```
polypulse/
├── backend/                  # Rust/Axum backend
│   ├── src/
│   │   ├── routes/           # API endpoints
│   │   ├── models/           # Database models
│   │   ├── services/         # Business logic
│   │   └── middleware/       # Auth, CORS, logging, rate limiting
│   └── Cargo.toml
├── contracts/                # Stellar smart contracts (Soroban)
│   └── contracts/
│       ├── market/           # LMSR prediction market
│       └── challenge/        # 1v1 wager contract
├── frontend/                 # React + TypeScript frontend
│   ├── src/
│   │   ├── components/       # Navbar, Footer, ProtectedRoute, WalletModal
│   │   ├── context/          # Auth, Wallet contexts
│   │   ├── pages/            # All route pages
│   │   ├── config/           # Stellar, API config
│   │   ├── lib/              # Stellar helper
│   │   └── hooks/            # useWalletDetection, useWagers, useChat
│   └── package.json
├── migrations/               # SQL migrations
├── scripts/                  # Deployment and setup scripts
├── docker-compose.yml
├── .env.example              # Template — copy to .env
└── README.md
```

---

## Testing

### Backend Tests
```bash
cd backend
cargo test
```

### Contract Tests
```bash
cd contracts
cargo test
```

### Frontend Tests
```bash
cd frontend
npm test
```

---

## License

MIT © PolyPulse
