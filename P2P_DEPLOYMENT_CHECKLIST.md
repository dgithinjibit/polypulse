# PolyPulse P2P Betting — Deployment Checklist

## Prerequisites

- Rust + `wasm32-unknown-unknown` target installed
- Stellar CLI (`stellar` or `soroban`) installed
- A funded Stellar testnet account (deployer key)
- PostgreSQL and Redis accessible from the backend host
- Docker (for backend deployment)

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Add WASM target
rustup target add wasm32-unknown-unknown

# Create and fund a testnet deployer key
stellar keys generate --global deployer --network testnet
curl "https://friendbot.stellar.org?addr=$(stellar keys address deployer)"
```

---

## Step 1 — Build the P2P Bet Smart Contract

```bash
cd contracts

# Build the p2p-bet contract to WASM
cargo build --release --target wasm32-unknown-unknown \
  -p p2p-bet

# Optimise the WASM (reduces deployment cost)
stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/p2p_bet.wasm
```

---

## Step 2 — Deploy P2P Bet Contract to Stellar Testnet

```bash
# Deploy and capture the contract ID
P2P_CONTRACT_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/p2p_bet.optimized.wasm \
  --source deployer \
  --network testnet)

echo "P2P Bet Contract ID: $P2P_CONTRACT_ID"

# Initialise the contract with your admin address
ADMIN_ADDRESS=$(stellar keys address deployer)

stellar contract invoke \
  --id "$P2P_CONTRACT_ID" \
  --source deployer \
  --network testnet \
  -- initialize \
  --admin "$ADMIN_ADDRESS"
```

Save the contract ID — you'll need it in the next step.

---

## Step 3 — Update Environment Variables

Copy `.env.example` to `.env` (backend) and set all values:

```bash
cp .env.example .env
```

### Required backend variables

| Variable | Description | Example |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://polypulse:pass@host:5432/polypulse` |
| `JWT_SECRET` | JWT signing secret (≥32 chars) | `openssl rand -base64 32` |
| `REDIS_URL` | Redis connection string | `redis://host:6379` |
| `STELLAR_P2P_BET_CONTRACT_ID` | Contract ID from Step 2 | `C...` |
| `ENCRYPTION_SECRET` | AES-256 key for shareable URLs | `openssl rand -base64 32` |
| `STELLAR_TREASURY_ADDRESS` | Receives 2% platform fees | Stellar G... address |
| `STELLAR_ADMIN_ADDRESS` | Resolves disputed bets | Stellar G... address |
| `STELLAR_RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `STELLAR_NETWORK` | `testnet` or `mainnet` | `testnet` |

### Required frontend variables (Vite)

| Variable | Description |
|---|---|
| `VITE_API_URL` | Backend API base URL |
| `VITE_WS_URL` | WebSocket URL |
| `VITE_STELLAR_NETWORK` | `testnet` or `mainnet` |
| `VITE_SOROBAN_RPC_URL` | Soroban RPC URL |
| `VITE_STELLAR_P2P_BET_CONTRACT_ID` | Contract ID from Step 2 |

---

## Step 4 — Run Database Migrations

```bash
# Using sqlx-cli directly
cd backend
sqlx migrate run --database-url "$DATABASE_URL"

# Or via Docker (migrations run automatically on container start via start.sh)
docker run --env-file ../.env polypulse-backend
```

Migrations are applied in order. The P2P-specific ones are:

- `20240301000001_p2p_bets.sql` — core p2p_bets, participants, outcome_reports tables
- `20240302000001_p2p_notifications.sql` — P2P notification events

---

## Step 5 — Deploy Backend to Staging

```bash
# Build the Docker image
docker build -t polypulse-backend:staging .

# Run with env file
docker run -d \
  --name polypulse-backend \
  --env-file .env \
  -p 8000:8000 \
  polypulse-backend:staging
```

For Render / Railway / Fly.io, set all env vars in the dashboard and trigger a deploy from the `main` branch.

---

## Step 6 — Deploy Frontend to Staging

```bash
cd frontend

# Set env vars (Vite bakes them in at build time)
cp ../.env.example .env.local
# Edit .env.local with staging values

# Build
npm run build

# Deploy dist/ to your static host (4EVERLAND, Vercel, Netlify, etc.)
# See 4EVERLAND_DEPLOYMENT.md for 4EVERLAND-specific steps
```

---

## Step 7 — Smoke Tests

Run these after deployment to verify the P2P betting stack is working end-to-end.

### Health check

```bash
curl https://api.staging.polypulse.co.ke/health
# Expected: {"status":"ok"}
```

### Create a bet (requires auth token)

```bash
TOKEN="<your-jwt-token>"
API="https://api.staging.polypulse.co.ke"

curl -X POST "$API/api/v1/p2p-bets" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "question": "Will it rain in Nairobi tomorrow?",
    "stake_amount": 1000000,
    "end_time": "'$(date -d '+1 day' -u +%s)'"
  }'
# Expected: {"id": <bet_id>, "shareable_url": "..."}
```

### List bets

```bash
curl "$API/api/v1/p2p-bets"
# Expected: JSON array of bets
```

### Resolve shareable URL

```bash
ENCRYPTED_ID="<encrypted_id_from_create_response>"
curl "$API/api/v1/p2p-bets/share/$ENCRYPTED_ID"
# Expected: full bet details
```

### WebSocket connectivity

```bash
# Using wscat (npm install -g wscat)
wscat -c "wss://api.staging.polypulse.co.ke/ws?token=$TOKEN"
# Send: {"type":"subscribe","bet_id":1}
# Expected: {"type":"subscribed","bet_id":1}
```

### Contract invocation (read-only)

```bash
stellar contract invoke \
  --id "$P2P_CONTRACT_ID" \
  --source deployer \
  --network testnet \
  -- get_bet \
  --bet_id 0
# Expected: None (no bets yet) or bet details
```

---

## Step 8 — End-to-End Verification Checklist

- [ ] Backend health endpoint returns `{"status":"ok"}`
- [ ] Can create a P2P bet via API
- [ ] Shareable URL is generated and resolves correctly
- [ ] Second user can join the bet
- [ ] WebSocket broadcasts participant-joined event
- [ ] Notification is delivered to bet creator
- [ ] Outcome can be reported after end_time
- [ ] Outcome confirmation triggers payout (or dispute)
- [ ] Admin can resolve a disputed bet
- [ ] Frontend loads and connects to correct contract ID
- [ ] Freighter wallet connects and signs transactions
- [ ] Transaction modal shows correct states (pending → confirmed)
- [ ] Position sidebar shows user's active bets

---

## Docker Compose (local staging simulation)

The existing `docker-compose` setup supports P2P betting. Ensure these services are running:

```bash
docker compose up -d postgres redis backend
```

The backend `start.sh` script runs migrations automatically before starting the server, so no manual migration step is needed in Docker.

---

## Rollback

If a deployment fails:

1. Revert the backend image tag in your hosting dashboard
2. The smart contract is immutable — if the contract has a critical bug, deploy a new contract and update `STELLAR_P2P_BET_CONTRACT_ID` + `VITE_STELLAR_P2P_BET_CONTRACT_ID`
3. Database migrations are forward-only; coordinate with the team before rolling back schema changes
