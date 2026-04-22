# Production Deployment Guide

Quick reference for deploying PolyPulse (Rust/Stellar) to production.

## Prerequisites

- Domain name
- SSL certificate
- Server with Docker (4GB RAM minimum)
- PostgreSQL database
- Redis instance
- Rust 1.79+ (for building)

## Environment Variables

Copy `.env.example` and configure:

```bash
# Backend
JWT_SECRET=your-jwt-secret-here
PORT=8000
DATABASE_URL=postgres://user:pass@host:5432/polypulse
REDIS_URL=redis://host:6379
CORS_ALLOWED_ORIGINS=https://yourdomain.com

# Stellar
STELLAR_NETWORK=mainnet
STELLAR_RPC_URL=https://soroban-mainnet.stellar.org
STELLAR_HORIZON_URL=https://horizon.stellar.org
STELLAR_MARKET_CONTRACT_ID=your-market-contract-id
STELLAR_CHALLENGE_CONTRACT_ID=your-challenge-contract-id

# Frontend
VITE_API_URL=https://api.yourdomain.com
VITE_WS_URL=wss://api.yourdomain.com
VITE_STELLAR_NETWORK=mainnet
VITE_STELLAR_HORIZON_URL=https://horizon.stellar.org
VITE_STELLAR_MARKET_CONTRACT_ID=your-market-contract-id
VITE_STELLAR_CHALLENGE_CONTRACT_ID=your-challenge-contract-id
```

## Deployment Steps

1. **Build Rust backend:**
   ```bash
   cd backend
   cargo build --release
   ```

2. **Run migrations:**
   ```bash
   cd backend
   sqlx migrate run
   ```

3. **Build frontend:**
   ```bash
   cd frontend
   npm install
   npm run build
   ```

4. **Start services with Docker:**
   ```bash
   docker compose up -d
   ```

## Soroban Contracts (Stellar)

1. **Build contracts:**
   ```bash
   cd contracts
   stellar contract build
   ```

2. **Deploy market contract:**
   ```bash
   stellar contract deploy \
     --wasm target/wasm32-unknown-unknown/release/market.wasm \
     --network mainnet \
     --source YOUR_SECRET_KEY
   ```

3. **Deploy challenge contract:**
   ```bash
   stellar contract deploy \
     --wasm target/wasm32-unknown-unknown/release/challenge.wasm \
     --network mainnet \
     --source YOUR_SECRET_KEY
   ```

4. **Update environment variables** with deployed contract IDs

## Monitoring

- Logs: `docker compose logs -f backend`
- Health: `curl https://yourdomain.com/health`
- Metrics: Configure Prometheus/Grafana

## Backup

```bash
# Database
docker compose exec db pg_dump -U polypulse polypulse > backup.sql

# Restore
docker compose exec -T db psql -U polypulse polypulse < backup.sql
```

## Security Checklist

- [ ] Change JWT_SECRET (use `openssl rand -base64 32`)
- [ ] Configure CORS_ALLOWED_ORIGINS
- [ ] Enable HTTPS
- [ ] Set up firewall
- [ ] Configure rate limiting
- [ ] Secure Redis (password, bind to localhost)
- [ ] Secure PostgreSQL (strong password, SSL)
- [ ] Regular backups
- [ ] Monitor logs
- [ ] Keep Rust dependencies updated

## Support

- Issues: GitHub Issues
- Docs: README.md
- Specs: `.kiro/specs/`
