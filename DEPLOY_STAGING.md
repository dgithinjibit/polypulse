# Staging Deployment Guide

This guide covers deploying the Stellar-Only Platform to a staging environment for testing before production.

## Prerequisites

- Staging server with Docker and Docker Compose installed
- Domain name for staging (e.g., staging.polypulse.co.ke)
- SSL certificate for staging domain
- Access to staging server via SSH
- PostgreSQL and Redis (via Docker)

## Pre-Deployment Checklist

- [ ] All Phase 1-6 tasks completed
- [ ] Production build tested locally (Task 9.2.5)
- [ ] Environment variables configured for staging
- [ ] Staging domain DNS configured
- [ ] SSL certificate obtained for staging domain
- [ ] Backup of current staging environment (if exists)

## Deployment Steps

### 1. Prepare Staging Environment Variables

Copy `.env.staging` to `.env` on the staging server:

```bash
# On staging server
cp .env.staging .env
```

Update the following variables in `.env`:
- `JWT_SECRET` - Generate with: `openssl rand -base64 32`
- `POSTGRES_PASSWORD` - Use a strong password
- `CORS_ALLOWED_ORIGINS` - Set to your staging domain
- `VITE_API_URL` - Set to your staging API URL
- `VITE_WS_URL` - Set to your staging WebSocket URL
- `STELLAR_MARKET_CONTRACT_ID` - Deploy contracts first (see step 3)
- `STELLAR_CHALLENGE_CONTRACT_ID` - Deploy contracts first (see step 3)

### 2. Build and Push Docker Images (Optional)

If using a container registry:

```bash
# Build images
docker compose -f docker-compose.staging.yml build

# Tag images
docker tag polypulse-backend:latest your-registry/polypulse-backend:staging
docker tag polypulse-frontend:latest your-registry/polypulse-frontend:staging

# Push to registry
docker push your-registry/polypulse-backend:staging
docker push your-registry/polypulse-frontend:staging
```

### 3. Deploy Stellar Contracts to Testnet

```bash
cd contracts

# Build contracts
stellar contract build

# Deploy market contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market.wasm \
  --network testnet \
  --source YOUR_TESTNET_SECRET_KEY

# Save the contract ID and update .env:
# STELLAR_MARKET_CONTRACT_ID=<contract-id>
# VITE_STELLAR_MARKET_CONTRACT_ID=<contract-id>

# Deploy challenge contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/challenge.wasm \
  --network testnet \
  --source YOUR_TESTNET_SECRET_KEY

# Save the contract ID and update .env:
# STELLAR_CHALLENGE_CONTRACT_ID=<contract-id>
# VITE_STELLAR_CHALLENGE_CONTRACT_ID=<contract-id>
```

### 4. Deploy to Staging Server

```bash
# SSH into staging server
ssh user@staging.polypulse.co.ke

# Clone or pull latest code
git clone https://github.com/your-org/polypulse.git
# OR
cd polypulse && git pull origin main

# Copy environment file
cp .env.staging .env
# Edit .env with actual values

# Start services
docker compose -f docker-compose.staging.yml up -d

# Check logs
docker compose -f docker-compose.staging.yml logs -f
```

### 5. Run Database Migrations

```bash
# On staging server
docker compose -f docker-compose.staging.yml exec backend \
  sqlx migrate run
```

### 6. Verify Deployment

Check the following:

```bash
# Health check
curl https://staging-api.polypulse.co.ke/health

# Check backend logs
docker compose -f docker-compose.staging.yml logs backend

# Check frontend logs
docker compose -f docker-compose.staging.yml logs frontend

# Check database connection
docker compose -f docker-compose.staging.yml exec db \
  psql -U polypulse -d polypulse_staging -c "SELECT 1;"
```

### 7. Configure Reverse Proxy (Nginx/Caddy)

Example Nginx configuration:

```nginx
# /etc/nginx/sites-available/staging.polypulse.co.ke

server {
    listen 80;
    server_name staging.polypulse.co.ke;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name staging.polypulse.co.ke;

    ssl_certificate /etc/letsencrypt/live/staging.polypulse.co.ke/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/staging.polypulse.co.ke/privkey.pem;

    location / {
        proxy_pass http://localhost:5173;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}

server {
    listen 443 ssl http2;
    server_name staging-api.polypulse.co.ke;

    ssl_certificate /etc/letsencrypt/live/staging-api.polypulse.co.ke/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/staging-api.polypulse.co.ke/privkey.pem;

    location / {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Reload Nginx:
```bash
sudo nginx -t
sudo systemctl reload nginx
```

## Post-Deployment Testing (Task 9.2.7)

### Manual Testing Checklist

1. **Frontend Access**
   - [ ] Visit https://staging.polypulse.co.ke
   - [ ] Verify page loads without errors
   - [ ] Check browser console for errors

2. **Wallet Connection**
   - [ ] Click "Connect Wallet" button
   - [ ] Verify Freighter wallet prompt appears
   - [ ] Connect wallet successfully
   - [ ] Verify wallet address displays correctly
   - [ ] Verify balance displays correctly

3. **Authentication Flow**
   - [ ] Connect wallet from login page
   - [ ] Verify signature request appears
   - [ ] Sign authentication message
   - [ ] Verify redirect to /social-login
   - [ ] Verify JWT tokens stored in localStorage

4. **Protected Routes**
   - [ ] Navigate to protected routes while connected
   - [ ] Verify access granted
   - [ ] Disconnect wallet
   - [ ] Verify redirect to /login

5. **API Endpoints**
   - [ ] Test POST /auth/stellar-nonce/
   - [ ] Test POST /auth/stellar-login/
   - [ ] Test authenticated endpoints
   - [ ] Verify CORS headers

6. **WebSocket Connection**
   - [ ] Verify WebSocket connects successfully
   - [ ] Test real-time updates (if applicable)

7. **Mobile Testing**
   - [ ] Test on mobile browser
   - [ ] Verify responsive design
   - [ ] Test wallet connection on mobile

8. **Error Handling**
   - [ ] Test with Freighter not installed
   - [ ] Test user rejection of connection
   - [ ] Test user rejection of signature
   - [ ] Test network errors
   - [ ] Verify error messages are user-friendly

### Automated Testing

```bash
# Run integration tests against staging
VITE_API_URL=https://staging-api.polypulse.co.ke npm run test:e2e

# Run smoke tests
npm run test:smoke -- --env=staging
```

## Monitoring

### Check Logs

```bash
# Backend logs
docker compose -f docker-compose.staging.yml logs -f backend

# Frontend logs
docker compose -f docker-compose.staging.yml logs -f frontend

# Database logs
docker compose -f docker-compose.staging.yml logs -f db

# All logs
docker compose -f docker-compose.staging.yml logs -f
```

### Health Checks

```bash
# Backend health
curl https://staging-api.polypulse.co.ke/health

# Database health
docker compose -f docker-compose.staging.yml exec db \
  pg_isready -U polypulse -d polypulse_staging
```

### Performance Monitoring

```bash
# Check response times
curl -w "@curl-format.txt" -o /dev/null -s https://staging.polypulse.co.ke

# Monitor resource usage
docker stats
```

## Rollback Procedure

If issues are found:

```bash
# Stop services
docker compose -f docker-compose.staging.yml down

# Restore database backup (if needed)
docker compose -f docker-compose.staging.yml exec -T db \
  psql -U polypulse polypulse_staging < backup.sql

# Checkout previous version
git checkout <previous-commit>

# Rebuild and restart
docker compose -f docker-compose.staging.yml up -d --build
```

## Common Issues

### Issue: Frontend can't connect to backend
**Solution**: Check CORS_ALLOWED_ORIGINS includes staging domain

### Issue: Wallet connection fails
**Solution**: Verify VITE_STELLAR_NETWORK is set to 'testnet'

### Issue: Authentication fails
**Solution**: Check JWT_SECRET is set and backend logs for errors

### Issue: Database connection fails
**Solution**: Verify DATABASE_URL and check database is running

## Next Steps

After successful staging deployment and testing:
1. Document any issues found
2. Fix issues and redeploy to staging
3. Get stakeholder approval
4. Proceed to production deployment (Task 9.2.8)

## Support

- Staging URL: https://staging.polypulse.co.ke
- API URL: https://staging-api.polypulse.co.ke
- Logs: `docker compose -f docker-compose.staging.yml logs -f`
- Issues: GitHub Issues
