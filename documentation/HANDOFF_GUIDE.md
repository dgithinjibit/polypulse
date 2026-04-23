# PolyPulse — Complete Handoff Documentation

**Last Updated**: April 23, 2026  
**Status**: Production-ready with known limitations  
**Live URLs**:
- Frontend (4everland): `https://polypulse-nfphmvqb-dgithinjibit.ipfs.4everland.app`
- Backend (Render): `https://polypulse-backend-436v.onrender.com`
- Repository: `https://github.com/dgithinjibit/polypulse`

---

## Table of Contents

1. [What is PolyPulse?](#what-is-polypulse)
2. [Architecture Overview](#architecture-overview)
3. [Tech Stack Deep Dive](#tech-stack-deep-dive)
4. [What's Working](#whats-working)
5. [What's Not Working / Needs Work](#whats-not-working--needs-work)
6. [Deployment Status](#deployment-status)
7. [Key Files & Directories](#key-files--directories)
8. [Database Schema](#database-schema)
9. [Authentication Flow](#authentication-flow)
10. [Smart Contracts](#smart-contracts)
11. [Known Issues & Workarounds](#known-issues--workarounds)
12. [Future Roadmap](#future-roadmap)
13. [How to Continue Development](#how-to-continue-development)

---

## What is PolyPulse?

PolyPulse is a **decentralized prediction markets platform** built on the Stellar blockchain. Users can:

- **Trade on real-world outcomes** (sports, politics, crypto prices, etc.)
- **Create prediction markets** with multiple outcome options
- **Buy and sell shares** using LMSR (Logarithmic Market Scoring Rule) pricing
- **Challenge friends** to 1v1 wagers
- **Earn from correct predictions** when markets resolve

Think Polymarket or Kalshi, but on Stellar with wallet-based authentication (no email/password required).

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         FRONTEND                            │
│  React 18 + TypeScript + Vite + Tailwind CSS               │
│  Hosted on: 4everland IPFS                                  │
│  URL: https://polypulse-*.ipfs.4everland.app               │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       │ HTTPS/WSS
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                         BACKEND                             │
│  Rust + Axum + SQLx + Redis                                 │
│  Hosted on: Render.com                                      │
│  URL: https://polypulse-backend-436v.onrender.com          │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
        ▼              ▼              ▼
┌───────────┐  ┌──────────────┐  ┌──────────────┐
│ PostgreSQL│  │    Redis     │  │   Stellar    │
│  Database │  │    Cache     │  │  Blockchain  │
│           │  │  + Pub/Sub   │  │  (Testnet)   │
└───────────┘  └──────────────┘  └──────────────┘
```

### Data Flow

1. **User connects wallet** (Freighter/Albedo) → Frontend requests nonce from backend
2. **User signs message** → Frontend sends signature to backend
3. **Backend verifies signature** → Issues JWT tokens (access + refresh)
4. **User trades** → Frontend calls API with JWT → Backend updates DB + broadcasts via WebSocket
5. **Real-time updates** → Redis Pub/Sub → WebSocket → All connected clients

---

## Tech Stack Deep Dive

### Frontend (`frontend/`)

| Technology | Version | Purpose |
|------------|---------|---------|
| **React** | 18.3.1 | UI framework |
| **TypeScript** | 5.6.2 | Type safety |
| **Vite** | 5.4.11 | Build tool (fast HMR) |
| **Tailwind CSS** | 3.4.15 | Utility-first styling |
| **React Router** | 6.28.0 | Client-side routing |
| **Axios** | 1.7.7 | HTTP client |
| **Stellar SDK** | 12.3.0 | Blockchain interactions |
| **Recharts** | 2.14.1 | Price charts |
| **Lucide React** | 0.462.0 | Icons |
| **Sonner** | 1.7.1 | Toast notifications |

**Key Features**:
- Aurora-themed gradient UI (purple/blue/teal)
- Responsive design (mobile-first)
- WebSocket real-time updates
- Lazy-loaded Stellar SDK (code splitting)
- Protected routes with auth context

### Backend (`backend/`)

| Technology | Version | Purpose |
|------------|---------|---------|
| **Rust** | 1.79+ | Systems language (fast, safe) |
| **Axum** | 0.7 | Async web framework |
| **SQLx** | 0.8 | Async PostgreSQL driver |
| **Redis** | 0.27 | Caching + Pub/Sub |
| **Tokio** | 1.41 | Async runtime |
| **Tower-HTTP** | 0.6 | Middleware (CORS, compression, tracing) |
| **JWT** | 9.3 | Token-based auth |
| **ed25519-dalek** | 2.1 | Stellar signature verification |

**Key Features**:
- RESTful API with 40+ endpoints
- WebSocket hub for real-time updates
- JWT auth with refresh token rotation
- Rate limiting (Redis sliding window)
- LMSR pricing algorithm
- Background task for auto-closing expired polls
- Comprehensive error handling

### Database (`backend/migrations/`)

| Technology | Version | Purpose |
|------------|---------|---------|
| **PostgreSQL** | 15+ | Primary data store |
| **SQLx Migrations** | — | Schema versioning |

**Tables**:
- `users` — User accounts (email, wallet address, balance)
- `polls` — Prediction markets
- `poll_options` — Outcome options for each poll
- `bets` — User positions in markets
- `challenges` — 1v1 wagers
- `wagers` — Peer-to-peer bets
- `comments` — Poll comments
- `notifications` — User notifications
- `wallet_transactions` — Transaction history
- `auth_nonces` — Single-use nonces for wallet auth
- `sessions` — JWT refresh tokens

### Smart Contracts (`contracts/`)

| Contract | Language | Purpose |
|----------|----------|---------|
| **Market** | Rust (Soroban) | LMSR prediction markets on-chain |
| **Challenge** | Rust (Soroban) | 1v1 wager escrow |

**Status**: Contracts are written and tested, but **not yet deployed** to Stellar testnet. Currently, all trading logic runs off-chain in the backend.

---

## What's Working

### ✅ Fully Functional

1. **Authentication**
   - Freighter wallet login (ed25519 signature verification)
   - JWT access tokens (30 min) + refresh tokens (7 days)
   - Token rotation on refresh
   - Nonce-based replay attack prevention

2. **Prediction Markets**
   - Create markets with 2-10 outcome options
   - LMSR pricing (buy/sell shares)
   - Real-time price updates
   - Market resolution (admin only)
   - Market cancellation with refunds

3. **Trading**
   - Buy shares (LMSR calculates price)
   - Sell shares (instant liquidity)
   - Portfolio view (open positions)
   - Transaction history

4. **Challenges & Wagers**
   - Create 1v1 challenges
   - Accept challenges
   - Resolve challenges
   - Share wager links

5. **Real-Time Features**
   - WebSocket connections
   - Live price updates
   - Live comment feed
   - Notification system

6. **UI/UX**
   - Responsive design (mobile/tablet/desktop)
   - Aurora gradient theme
   - Toast notifications
   - Loading states
   - Error handling

7. **Security**
   - CORS protection
   - CSP headers
   - Rate limiting
   - SQL injection prevention (parameterized queries)
   - XSS protection (React escaping)

8. **Deployment**
   - Frontend on 4everland IPFS
   - Backend on Render.com
   - Auto-deploy on git push
   - Environment-based config

---

## What's Not Working / Needs Work

### ❌ Known Issues

1. **Smart Contracts Not Deployed**
   - Contracts are written but not deployed to Stellar testnet
   - All trading currently happens off-chain in the backend
   - **Impact**: Not truly decentralized yet
   - **Fix**: Deploy contracts and integrate with frontend

2. **M-Pesa Integration Disabled**
   - Code exists but commented out
   - Was intended for Kenyan mobile money deposits
   - **Impact**: Users can't deposit fiat
   - **Fix**: Re-enable and test with Safaricom Daraja API

3. **Email Verification Not Tested**
   - Code exists for email verification
   - SMTP config not set up
   - **Impact**: Users can register but emails aren't sent
   - **Fix**: Configure SMTP (Gmail, SendGrid, etc.)

4. **Paymaster Not Fully Implemented**
   - Gasless transaction relay exists but untested
   - **Impact**: Users pay gas fees directly
   - **Fix**: Test and deploy paymaster service

5. **No Admin Dashboard**
   - Market resolution requires manual API calls
   - No UI for admin actions
   - **Impact**: Hard to manage markets
   - **Fix**: Build admin panel

6. **Limited Error Messages**
   - Some API errors return generic messages
   - **Impact**: Hard to debug for users
   - **Fix**: Add more specific error codes

7. **No User Profiles**
   - Can't view other users' profiles
   - No leaderboard
   - **Impact**: Less social engagement
   - **Fix**: Add user profile pages

8. **No Search/Filters**
   - Can't search markets by keyword
   - Limited filtering options
   - **Impact**: Hard to find specific markets
   - **Fix**: Add search bar and advanced filters

---

## Deployment Status

### Frontend (4everland)

**Status**: ✅ Deployed and working

**URL**: `https://polypulse-nfphmvqb-dgithinjibit.ipfs.4everland.app`

**Config**:
- Auto-deploys from `main` branch
- Build command: `npm run build`
- Root directory: `frontend`
- Node version: 20.x

**Environment Variables**:
```bash
VITE_API_URL=https://polypulse-backend-436v.onrender.com
VITE_API_HOST=polypulse-backend-436v.onrender.com
VITE_WS_URL=wss://polypulse-backend-436v.onrender.com
VITE_STELLAR_NETWORK=testnet
VITE_HORIZON_URL=https://horizon-testnet.stellar.org
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

**Known Issue**: 4everland generates a new hash on every deployment, changing the URL. We fixed this by allowing all `*.ipfs.4everland.app` origins in the backend CORS config.

### Backend (Render)

**Status**: ✅ Deployed and working

**URL**: `https://polypulse-backend-436v.onrender.com`

**Config**:
- Auto-deploys from `main` branch
- Build command: `cargo build --release`
- Start command: `./target/release/backend`
- Instance type: Free tier (spins down after 15 min inactivity)

**Environment Variables**:
```bash
JWT_SECRET=<redacted>
DATABASE_URL=<redacted>
REDIS_URL=<redacted>
CORS_ORIGINS=http://localhost:5173,http://localhost:3000
RUST_PORT=8000
FRONTEND_URL=https://polypulse-nfphmvqb-dgithinjibit.ipfs.4everland.app
```

**Known Issue**: Free tier spins down after inactivity, causing 30-second cold starts. Upgrade to paid tier for always-on.

### Database (Render PostgreSQL)

**Status**: ✅ Running

**Version**: PostgreSQL 15

**Migrations**: All applied (7 migrations)

**Backup**: Render handles automatic backups on paid tier

### Redis (Render Redis)

**Status**: ✅ Running

**Version**: Redis 7

**Usage**: Caching + Pub/Sub for WebSockets

---

## Key Files & Directories

### Frontend

```
frontend/
├── src/
│   ├── components/          # Reusable UI components
│   │   ├── Navbar.tsx       # Top navigation with wallet connect
│   │   ├── Footer.tsx       # Footer with links
│   │   ├── WalletModal.tsx  # Wallet connection modal
│   │   └── ProtectedRoute.tsx  # Auth guard for routes
│   ├── context/             # React context providers
│   │   ├── AuthContext.tsx  # JWT auth state
│   │   └── StellarWalletContext.tsx  # Wallet connection state
│   ├── pages/               # Route components
│   │   ├── Home.tsx         # Landing page
│   │   ├── Markets.tsx      # List all markets
│   │   ├── MarketDetail.tsx # Single market view
│   │   ├── Challenges.tsx   # List challenges
│   │   ├── Wagers.tsx       # List wagers
│   │   ├── Portfolio.tsx    # User positions
│   │   ├── Wallet.tsx       # Wallet balance/transactions
│   │   ├── Login.tsx        # Email/password login (legacy)
│   │   └── SocialLogin.tsx  # Wallet login
│   ├── services/            # API client
│   │   └── api.ts           # Axios instance with interceptors
│   ├── lib/                 # Utilities
│   │   ├── stellar-helper.ts  # Stellar SDK wrapper
│   │   └── utils.ts         # General utilities
│   ├── hooks/               # Custom React hooks
│   │   ├── useWagers.ts     # Wager data fetching
│   │   └── use-toast.ts     # Toast notifications
│   ├── types/               # TypeScript types
│   │   └── index.ts         # Shared types
│   └── main.tsx             # App entry point
├── index.html               # HTML template (with CSP meta tag)
├── vite.config.ts           # Vite build config
└── package.json             # Dependencies
```

### Backend

```
backend/
├── src/
│   ├── routes/              # HTTP route handlers
│   │   ├── mod.rs           # Router assembly + CORS config
│   │   ├── auth.rs          # Login, register, wallet auth
│   │   ├── polls.rs         # Market CRUD
│   │   ├── bets.rs          # Trading endpoints
│   │   ├── challenges.rs    # Challenge endpoints
│   │   ├── wagers.rs        # Wager endpoints
│   │   ├── wallet.rs        # Balance, transactions
│   │   ├── notifications.rs # Notification endpoints
│   │   └── ...
│   ├── services/            # Business logic
│   │   ├── cache.rs         # Redis caching
│   │   ├── wallet.rs        # Wallet operations
│   │   ├── paymaster.rs     # Gasless transactions
│   │   ├── poll_closer.rs   # Background task for auto-closing
│   │   └── ...
│   ├── middleware/          # HTTP middleware
│   │   ├── auth.rs          # JWT verification
│   │   ├── rate_limit.rs    # Rate limiting
│   │   ├── request_id.rs    # Request ID generation
│   │   └── validation.rs    # Input validation
│   ├── ws/                  # WebSocket
│   │   └── mod.rs           # WebSocket hub
│   ├── models.rs            # Database models
│   ├── lmsr.rs              # LMSR pricing algorithm
│   ├── config.rs            # Environment config
│   ├── state.rs             # Shared app state
│   ├── db.rs                # Database connection
│   ├── errors.rs            # Error types
│   └── main.rs              # Entry point
├── migrations/              # SQL migrations
│   ├── 20240101000001_initial_schema.sql
│   ├── 20240102000001_gasless_transactions.sql
│   └── ...
└── Cargo.toml               # Dependencies
```

### Smart Contracts

```
contracts/
└── contracts/
    ├── market/              # LMSR market contract
    │   ├── src/
    │   │   ├── lib.rs       # Contract entry points
    │   │   └── storage.rs   # Storage helpers
    │   └── Cargo.toml
    └── challenge/           # 1v1 wager contract
        ├── src/
        │   └── lib.rs       # Contract entry points
        └── Cargo.toml
```

---

## Database Schema

### Core Tables

**users**
```sql
id SERIAL PRIMARY KEY
email VARCHAR(255) UNIQUE
password_hash VARCHAR(255)
stellar_public_key VARCHAR(56) UNIQUE
balance BIGINT DEFAULT 0
created_at TIMESTAMPTZ
```

**polls** (prediction markets)
```sql
id SERIAL PRIMARY KEY
title VARCHAR(255)
description TEXT
category VARCHAR(50)
creator_id INTEGER REFERENCES users(id)
status VARCHAR(20)  -- open, closed, resolved, cancelled
close_time TIMESTAMPTZ
liquidity BIGINT
created_at TIMESTAMPTZ
```

**poll_options** (outcome options)
```sql
id SERIAL PRIMARY KEY
poll_id INTEGER REFERENCES polls(id)
option_text VARCHAR(255)
shares BIGINT DEFAULT 0
```

**bets** (user positions)
```sql
id SERIAL PRIMARY KEY
user_id INTEGER REFERENCES users(id)
poll_id INTEGER REFERENCES polls(id)
option_id INTEGER REFERENCES poll_options(id)
shares BIGINT
cost BIGINT
created_at TIMESTAMPTZ
```

**challenges** (1v1 wagers)
```sql
id SERIAL PRIMARY KEY
creator_id INTEGER REFERENCES users(id)
opponent_id INTEGER REFERENCES users(id)
title VARCHAR(255)
stake BIGINT
status VARCHAR(20)  -- open, accepted, resolved, cancelled
winner_id INTEGER REFERENCES users(id)
created_at TIMESTAMPTZ
```

### Supporting Tables

- `wagers` — Peer-to-peer bets
- `comments` — Poll comments
- `notifications` — User notifications
- `wallet_transactions` — Transaction history
- `auth_nonces` — Single-use nonces for wallet auth
- `sessions` — JWT refresh tokens

---

## Authentication Flow

### Wallet-Based Auth (Freighter)

```
┌─────────┐                 ┌─────────┐                 ┌─────────┐
│ Frontend│                 │ Backend │                 │ Freighter│
└────┬────┘                 └────┬────┘                 └────┬────┘
     │                           │                           │
     │ 1. POST /auth/stellar-nonce                          │
     │    { public_key }          │                           │
     ├──────────────────────────>│                           │
     │                           │                           │
     │ 2. Generate nonce         │                           │
     │    Store in DB            │                           │
     │                           │                           │
     │ 3. Return nonce           │                           │
     │<──────────────────────────┤                           │
     │                           │                           │
     │ 4. Request signature      │                           │
     ├───────────────────────────────────────────────────────>│
     │                           │                           │
     │                           │    5. User approves       │
     │                           │                           │
     │ 6. Return signature       │                           │
     │<───────────────────────────────────────────────────────┤
     │                           │                           │
     │ 7. POST /auth/stellar-login                          │
     │    { public_key, signature, message }                │
     ├──────────────────────────>│                           │
     │                           │                           │
     │                           │ 8. Verify signature       │
     │                           │    Mark nonce as used     │
     │                           │    Create/update user     │
     │                           │    Generate JWT tokens    │
     │                           │                           │
     │ 9. Return tokens          │                           │
     │    { access_token, refresh_token }                   │
     │<──────────────────────────┤                           │
     │                           │                           │
     │ 10. Store tokens          │                           │
     │     in localStorage       │                           │
     │                           │                           │
```

### Token Refresh Flow

```
┌─────────┐                 ┌─────────┐
│ Frontend│                 │ Backend │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. API call with expired token
     ├──────────────────────────>│
     │                           │
     │ 2. 401 Unauthorized       │
     │<──────────────────────────┤
     │                           │
     │ 3. POST /auth/refresh     │
     │    { refresh_token }      │
     ├──────────────────────────>│
     │                           │
     │                           │ 4. Verify refresh token
     │                           │    Rotate refresh token
     │                           │    Generate new access token
     │                           │
     │ 5. Return new tokens      │
     │<──────────────────────────┤
     │                           │
     │ 6. Retry original request │
     ├──────────────────────────>│
     │                           │
     │ 7. Success                │
     │<──────────────────────────┤
     │                           │
```

---

## Smart Contracts

### Market Contract (LMSR)

**File**: `contracts/contracts/market/src/lib.rs`

**Functions**:
- `create_market(title, options, liquidity)` — Create a new market
- `buy_shares(market_id, option_id, amount)` — Buy shares
- `sell_shares(market_id, option_id, amount)` — Sell shares
- `resolve_market(market_id, winning_option)` — Resolve market
- `claim_payout(market_id)` — Claim winnings
- `get_price(market_id, option_id)` — Get current price

**LMSR Pricing**:
```rust
// Cost function: C(q) = b * ln(sum(e^(q_i / b)))
// Price: p_i = e^(q_i / b) / sum(e^(q_j / b))
// where:
//   q_i = shares of option i
//   b = liquidity parameter
```

**Status**: ✅ Written and tested, ❌ Not deployed

### Challenge Contract

**File**: `contracts/contracts/challenge/src/lib.rs`

**Functions**:
- `create_challenge(opponent, stake, title)` — Create challenge
- `accept_challenge(challenge_id)` — Accept with matching stake
- `resolve_challenge(challenge_id, winner)` — Resolve and payout
- `cancel_challenge(challenge_id)` — Cancel and refund

**Status**: ✅ Written and tested, ❌ Not deployed

---

## Known Issues & Workarounds

### Issue 1: 4everland URL Changes on Every Deploy

**Problem**: IPFS gateway generates new hash, breaking CORS

**Workaround**: Backend now accepts all `*.ipfs.4everland.app` origins

**Permanent Fix**: Set up custom domain on 4everland

### Issue 2: Render Free Tier Cold Starts

**Problem**: Backend spins down after 15 min inactivity, causing 30s delays

**Workaround**: First request after inactivity will be slow

**Permanent Fix**: Upgrade to Render paid tier ($7/month)

### Issue 3: CSP Blocks `eval()` on IPFS

**Problem**: Node.js polyfills use `eval()`, blocked by strict CSP

**Workaround**: Excluded `vm` polyfill, added explicit CSP meta tag

**Status**: ✅ Fixed

### Issue 4: No Smart Contract Integration

**Problem**: Contracts written but not deployed

**Workaround**: All trading happens off-chain in backend

**Permanent Fix**: Deploy contracts and integrate with frontend

### Issue 5: No Fiat On-Ramp

**Problem**: Users need XLM to trade, but can't buy with fiat

**Workaround**: Users must buy XLM on exchanges

**Permanent Fix**: Integrate M-Pesa or Stripe

---

## Future Roadmap

### Phase 1: Core Improvements (1-2 weeks)

- [ ] Deploy smart contracts to Stellar testnet
- [ ] Integrate contracts with frontend
- [ ] Add admin dashboard for market management
- [ ] Improve error messages
- [ ] Add search and filters

### Phase 2: Social Features (2-3 weeks)

- [ ] User profiles
- [ ] Leaderboard
- [ ] Follow users
- [ ] Share markets on social media
- [ ] Comment likes/replies

### Phase 3: Advanced Trading (3-4 weeks)

- [ ] Limit orders
- [ ] Stop-loss orders
- [ ] Portfolio analytics
- [ ] Price alerts
- [ ] Trading bots API

### Phase 4: Monetization (4-6 weeks)

- [ ] Trading fees (1-2%)
- [ ] Market creation fees
- [ ] Premium features
- [ ] Referral program

### Phase 5: Mainnet Launch (6-8 weeks)

- [ ] Security audit
- [ ] Deploy to Stellar mainnet
- [ ] Custom domain
- [ ] Marketing campaign
- [ ] User onboarding flow

---

## How to Continue Development

### Local Setup

1. **Clone the repo**:
   ```bash
   git clone https://github.com/dgithinjibit/polypulse.git
   cd polypulse
   ```

2. **Set up environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your values
   ```

3. **Start services** (Docker):
   ```bash
   docker compose up -d postgres redis
   ```

4. **Run migrations**:
   ```bash
   cd backend
   sqlx migrate run
   ```

5. **Start backend**:
   ```bash
   cargo run
   # Runs on http://localhost:8000
   ```

6. **Start frontend**:
   ```bash
   cd frontend
   npm install
   npm run dev
   # Runs on http://localhost:5173
   ```

### Making Changes

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make changes and test locally**

3. **Commit and push**:
   ```bash
   git add .
   git commit -m "feat: your feature description"
   git push origin feature/your-feature-name
   ```

4. **Create pull request** on GitHub

5. **Merge to main** → Auto-deploys to 4everland and Render

### Testing

**Backend**:
```bash
cd backend
cargo test
```

**Frontend**:
```bash
cd frontend
npm test                # Unit tests
npm run test:e2e        # E2E tests (Playwright)
```

**Contracts**:
```bash
cd contracts
cargo test
```

### Deploying Smart Contracts

1. **Install Stellar CLI**:
   ```bash
   cargo install --locked stellar-cli
   ```

2. **Build contracts**:
   ```bash
   cd contracts
   stellar contract build
   ```

3. **Deploy to testnet**:
   ```bash
   stellar contract deploy \
     --wasm target/wasm32-unknown-unknown/release/market.wasm \
     --network testnet \
     --source <your-secret-key>
   ```

4. **Update environment variables** with contract IDs

5. **Test contract calls** from frontend

---

## Contact & Support

**Developer**: Daniel Githinji  
**Email**: dgithinjibit@gmail.com  
**GitHub**: https://github.com/dgithinjibit  
**Repository**: https://github.com/dgithinjibit/polypulse

---

## Final Notes

This project is **production-ready** for a hackathon demo or MVP launch, but needs work before handling real money at scale:

1. **Security audit** — Have a professional audit the smart contracts
2. **Load testing** — Test with 1000+ concurrent users
3. **Monitoring** — Set up error tracking (Sentry) and metrics (Grafana)
4. **Backups** — Automated database backups
5. **Legal** — Terms of service, privacy policy, compliance

The codebase is well-documented with comments explaining every function. If you're stuck, read the comments — they're written for junior devs.

Good luck! 🚀
