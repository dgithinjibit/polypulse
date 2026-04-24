# PolyPulse — Stellar Prediction Markets

A Web3 prediction markets platform where users trade on real-world outcomes using Stellar wallets. Built for the Stellar hackathon, live at [polypulse.co.ke](https://polypulse.co.ke).

## 🚀 New Features (v2.0)

- **Multi-Participant Betting Pools**: Unlimited participants per bet with proportional payouts
- **Activity Feed**: Real-time activity tracking and trending bets
- **Bet Templates**: Quick bet creation with pre-filled templates (Crypto, Sports, Weather)
- **Reputation System**: User trust scores (0-100) with badges and verified status
- **Gamification**: XP system, levels (Bronze/Silver/Gold/Diamond), achievements, leaderboards

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 18 + TypeScript + Vite + Tailwind CSS |
| Backend | Rust + Axum + SQLx |
| Realtime | WebSockets via Axum + Redis Pub/Sub |
| Blockchain | Stellar (Soroban Smart Contracts) |
| Database | PostgreSQL 15 |
| Cache | Redis 7 |
| Auth | JWT + Freighter wallet signature verification (ed25519) |
| Testing | Vitest + Playwright (frontend), cargo test (backend/contracts) |

---

## Project Structure

```
polypulse/
├── backend/                        # Rust/Axum API server
│   ├── src/
│   │   ├── routes/                 # HTTP route handlers (auth, polls, bets, wagers, wallet…)
│   │   ├── services/               # Business logic (cache, wallet, paymaster, poll_closer…)
│   │   │   ├── activity_feed.rs    # Activity tracking and trending bets
│   │   │   ├── bet_templates.rs    # Bet template system
│   │   │   ├── leaderboard.rs      # XP, levels, and leaderboards
│   │   │   ├── reputation.rs       # User reputation scoring
│   │   │   └── telegram_bot.rs     # Telegram bot integration
│   │   ├── middleware/             # Auth, rate limiting, request ID, validation
│   │   ├── models.rs               # Database model structs
│   │   ├── lmsr.rs                 # LMSR pricing algorithm
│   │   ├── config.rs               # Environment-based configuration
│   │   ├── state.rs                # Shared AppState (DB pool, Redis, config)
│   │   └── ws/                     # WebSocket hub for real-time updates
│   ├── migrations/                 # SQL migration files (sqlx)
│   ├── tests/                      # Integration and property-based tests
│   └── Cargo.toml
├── contracts/                      # Stellar Soroban smart contracts
│   └── contracts/
│       ├── market/                 # LMSR prediction market contract
│       ├── challenge/              # 1v1 wager contract
│       └── multi-pool/             # Multi-participant betting pool contract
├── frontend/                       # React + TypeScript SPA
│   ├── src/
│   │   ├── components/             # Navbar, Footer, WalletModal, ProtectedRoute…
│   │   │   └── Leaderboard.tsx     # Leaderboard component
│   │   ├── context/                # AuthContext, StellarWalletContext
│   │   ├── pages/                  # Markets, Challenges, Wagers, Portfolio, Wallet…
│   │   ├── hooks/                  # useWagers, useChat, use-toast
│   │   ├── services/               # Axios API client
│   │   │   └── pwa.ts              # PWA service worker integration
│   │   ├── lib/                    # stellar-helper, stellar-sdk-loader, utils
│   │   ├── config/                 # API config
│   │   └── types/                  # Shared TypeScript types
│   ├── public/
│   │   ├── sw.js                   # Service worker for PWA
│   │   └── manifest.json           # PWA manifest
│   ├── e2e/                        # Playwright end-to-end tests
│   └── package.json
├── docker-compose.yml
├── Dockerfile
├── .env.example
├── ENHANCEMENT_IMPLEMENTATION_STATUS.md  # Feature implementation tracking
└── README.md
```

---

## Quick Start

### Prerequisites

- Rust 1.79+
- Node.js 18+
- PostgreSQL 15+
- Redis 7+
- Stellar CLI (for contract deployment only)

### 1. Environment

```bash
cp .env.example .env
# Edit .env — set at minimum: JWT_SECRET, DATABASE_URL, POSTGRES_PASSWORD
```

### 2. Backend

```bash
cd backend

# Run DB migrations (includes new enhancement tables)
sqlx migrate run

# Start dev server (listens on :8000)
cargo run
```

### 3. Frontend

```bash
cd frontend
npm install
npm run dev
# Runs at http://localhost:5173
```

### 4. Docker (all services)

```bash
docker compose up -d
```

This starts PostgreSQL, Redis, the Rust backend, and the frontend together.

---

## Environment Variables

### Backend / Root `.env`

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | Yes | — | JWT signing secret. Generate: `openssl rand -base64 32` |
| `DATABASE_URL` | Yes | — | PostgreSQL URL: `postgres://user:pass@host:5432/polypulse` |
| `POSTGRES_DB` | Yes | `polypulse` | Database name (used by Docker) |
| `POSTGRES_USER` | Yes | `polypulse` | Database user (used by Docker) |
| `POSTGRES_PASSWORD` | Yes | — | Database password |
| `REDIS_URL` | No | `redis://127.0.0.1:6379` | Redis connection URL |
| `RUST_PORT` | No | `8000` | Backend server port |
| `CORS_ORIGINS` | No | `http://localhost:5173` | Comma-separated allowed origins |
| `STELLAR_NETWORK` | No | — | `mainnet` or `testnet` |
| `STELLAR_RPC_URL` | No | — | Soroban RPC endpoint |
| `STELLAR_HORIZON_URL` | No | — | Horizon API endpoint |
| `STELLAR_MARKET_CONTRACT_ID` | No | — | Deployed market contract address |
| `STELLAR_CHALLENGE_CONTRACT_ID` | No | — | Deployed challenge contract address |

### Frontend `frontend/.env.local`

| Variable | Description |
|---|---|
| `VITE_API_URL` | Backend base URL, e.g. `https://api.polypulse.co.ke` |
| `VITE_API_HOST` | Backend host for WebSocket URL construction |
| `VITE_WS_URL` | WebSocket base URL, e.g. `wss://api.polypulse.co.ke` |
| `VITE_STELLAR_NETWORK` | `mainnet` or `testnet` |
| `VITE_HORIZON_URL` | Horizon API URL |
| `VITE_SOROBAN_RPC_URL` | Soroban RPC URL |
| `VITE_STELLAR_MARKET_CONTRACT_ID` | Market contract address |
| `VITE_STELLAR_CHALLENGE_CONTRACT_ID` | Challenge contract address |

---

## API Reference

All endpoints are prefixed with `/api/v1`. WebSocket connects at `/ws?token=<jwt>`.

### Authentication

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `POST` | `/auth/register` | No | Register with email/password |
| `POST` | `/auth/login` | No | Login with email/password |
| `POST` | `/auth/stellar-nonce` | No | Get nonce for Stellar wallet auth |
| `POST` | `/auth/stellar-login` | No | Authenticate with Freighter signature |
| `POST` | `/auth/refresh` | No | Refresh access token |
| `GET` | `/auth/verify-email/:token` | No | Verify email address |
| `POST` | `/auth/logout` | Yes | Invalidate session |

### Prediction Markets (Polls)

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `GET` | `/polls` | No | List all markets |
| `GET` | `/polls/:id` | No | Get market details |
| `POST` | `/polls` | Yes | Create a market |
| `POST` | `/polls/:id/resolve` | Yes | Resolve with winning outcome |
| `POST` | `/polls/:id/suspend` | Yes | Suspend a market |
| `POST` | `/polls/:id/cancel` | Yes | Cancel and refund |

### Trading

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `POST` | `/bets` | Yes | Buy shares (LMSR pricing) |
| `POST` | `/bets/sell` | Yes | Sell shares |
| `GET` | `/positions` | Yes | Get open positions |
| `GET` | `/markets/:id/prices` | No | Current option prices |
| `GET` | `/markets/:id/history` | No | Price history |

### Challenges & Wagers

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `GET` | `/challenges` | No | List challenges |
| `GET` | `/challenges/:id` | No | Challenge details |
| `POST` | `/challenges` | Yes | Create challenge |
| `POST` | `/challenges/:id/accept` | Yes | Accept challenge |
| `POST` | `/challenges/:id/resolve` | Yes | Resolve challenge |
| `POST` | `/challenges/:id/cancel` | Yes | Cancel challenge |
| `GET` | `/wagers/:id` | No | Get wager by share link |
| `GET` | `/wagers` | Yes | List your wagers |
| `POST` | `/wagers` | Yes | Create wager |
| `POST` | `/wagers/:id/accept` | Yes | Accept wager |
| `POST` | `/wagers/:id/cancel` | Yes | Cancel wager |

### Wallet & Notifications

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `GET` | `/wallet/balance` | Yes | Get wallet balance |
| `GET` | `/wallet/transactions` | Yes | Transaction history |
| `GET` | `/notifications` | Yes | List notifications |
| `POST` | `/notifications/:id/read` | Yes | Mark as read |
| `POST` | `/notifications/read-all` | Yes | Mark all as read |
| `GET` | `/notifications/unread-count` | Yes | Unread count |

### Other

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `GET` | `/health` | No | Health check |
| `GET` | `/categories` | No | List poll categories |
| `GET` | `/users/me` | Yes | Current user profile |
| `GET` | `/users/me/portfolio` | Yes | User portfolio |
| `POST` | `/paymaster/relay` | Yes | Relay gasless transaction |

### Enhancements (v2.0)

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| `GET` | `/activities` | No | Get recent activities |
| `GET` | `/activities/trending` | No | Get trending bets |
| `GET` | `/activities/user/:id` | Yes | Get user activity history |
| `GET` | `/templates` | No | Get all bet templates |
| `GET` | `/templates/:category` | No | Get templates by category |
| `POST` | `/templates/:id/use` | Yes | Increment template usage |
| `GET` | `/reputation/:user_id` | No | Get user reputation |
| `GET` | `/reputation/:user_id/history` | Yes | Get reputation history |
| `GET` | `/leaderboard/:category` | No | Get leaderboard (top_earners, best_predictors, most_active) |
| `GET` | `/multi-pools` | No | List multi-participant pools |
| `GET` | `/multi-pools/:id` | No | Get pool details |
| `POST` | `/multi-pools` | Yes | Create multi-participant pool |
| `POST` | `/multi-pools/:id/join` | Yes | Join pool with position and stake |
| `GET` | `/multi-pools/:id/odds` | No | Get current odds |

---

## Authentication Flow

Wallet-based auth using [Freighter](https://freighter.app):

1. Frontend calls `POST /api/v1/auth/stellar-nonce` with the user's public key
2. User signs a message containing the nonce in Freighter
3. Frontend sends `{ public_key, signature, message }` to `POST /api/v1/auth/stellar-login`
4. Backend verifies the ed25519 signature and marks the nonce as used
5. Backend returns JWT access token (30 min) + refresh token (7 days)
6. Frontend attaches `Authorization: Bearer <token>` to all API requests
7. WebSocket connections authenticate via `?token=<jwt>` query param

---

## Rate Limits

Implemented via Redis sliding window per IP (anonymous) or user ID (authenticated).

| Tier | Limit | Endpoints |
|---|---|---|
| Anonymous | 60 req/min | Public read endpoints |
| Authenticated | 300 req/min | Protected endpoints |
| Auth | 10 req/min | `/auth/*` login/nonce endpoints |
| Trading | 30 req/min | `/bets`, `/markets/*/prices` |

---

## Smart Contracts

### Market Contract (`contracts/contracts/market/`)

Soroban contract implementing LMSR prediction markets.

- Create markets with multiple outcome options
- Buy/sell shares with LMSR pricing
- Resolve markets and distribute payouts
- XLM token integration

```bash
cd contracts
stellar contract build
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market.wasm \
  --network testnet
```

### Challenge Contract (`contracts/contracts/challenge/`)

Direct 1v1 wager contract.

- Create open or targeted challenges
- Accept with matching stake
- Resolve with winner determination
- Automatic payout distribution

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/challenge.wasm \
  --network testnet
```

### Multi-Pool Contract (`contracts/contracts/multi-pool/`) **NEW**

Multi-participant betting pool with proportional payouts.

- Unlimited participants per pool
- Separate Yes/No position tracking
- Proportional payout calculation (7% platform fee)
- Real-time odds calculation
- Property-based tested for mathematical correctness

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm \
  --network testnet
```

---

## Testing

### Backend

```bash
cd backend
cargo test                              # All tests
cargo test --test multi_pool_payout_tests  # Property-based payout tests
```

### Contracts

```bash
cd contracts
cargo test                              # All contract tests
cd contracts/multi-pool && cargo test  # Multi-pool specific tests
```

### Frontend (unit)

```bash
cd frontend
npm test
```

### Frontend (e2e)

```bash
cd frontend
npm run test:e2e
```

---

## Production Deployment

### Server Setup

```bash
sudo apt update && sudo apt install -y \
  build-essential pkg-config libssl-dev libpq-dev \
  nodejs npm redis-server postgresql

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Backend

```bash
cd backend
cargo build --release
sqlx migrate run
./target/release/backend
```

### Frontend

```bash
cd frontend
npm install
cp .env.production .env.local   # set VITE_API_URL
npm run build
# Serve frontend/dist/ with nginx or any static host
```

### Nginx

```nginx
server {
    listen 443 ssl;
    server_name polypulse.co.ke;

    root /var/www/polypulse/frontend/dist;
    index index.html;
    try_files $uri $uri/ /index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /ws {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

---

## Security

- JWT access tokens expire in **30 minutes**; refresh tokens in **7 days** with rotation
- WebSocket connections require a valid JWT — unauthenticated connections are rejected
- CORS is restricted to explicit origins — wildcard `*` is never used in production
- All security headers enabled (HSTS, XSS protection, CSP, X-Frame-Options)
- Wallet nonces are single-use and expire after **5 minutes**
- Rate limiting applied at the API layer via Redis sliding window
- PostgreSQL and Redis ports bound to `127.0.0.1` only in Docker

---

## License

MIT © PolyPulse
