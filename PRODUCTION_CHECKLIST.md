# Production Deployment Checklist

## Pre-Deployment

### Database
- [ ] Run all migrations: `sqlx migrate run`
- [ ] Verify new tables exist:
  - `activities`
  - `bet_templates`
  - `telegram_users`
  - `achievements`
  - `user_achievements`
  - `reputation_events`
  - `push_subscriptions`
- [ ] Verify user table columns added:
  - `xp`, `level`, `win_streak`
  - `reputation_score`, `is_verified`
  - `is_premium`, `premium_activated_at`
- [ ] Verify p2p_bets table columns added:
  - `is_multi_participant`
  - `total_yes_stakes`, `total_no_stakes`
  - `participant_count`
- [ ] Verify default data inserted:
  - 5 achievements
  - 4 bet templates

### Smart Contracts
- [ ] Build multi-pool contract: `cd contracts && stellar contract build`
- [ ] Deploy to testnet first for testing
- [ ] Deploy to mainnet: `stellar contract deploy --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm --network mainnet`
- [ ] Save contract ID to environment variables
- [ ] Test contract functions on testnet

### Backend
- [ ] Run all tests: `cargo test`
- [ ] Run property-based tests: `cargo test --test multi_pool_payout_tests`
- [ ] Verify all tests pass
- [ ] Build release binary: `cargo build --release`
- [ ] Update environment variables:
  - `STELLAR_MULTI_POOL_CONTRACT_ID`
- [ ] Test API endpoints locally

### Frontend
- [ ] Update `.env.production`:
  - `VITE_STELLAR_MULTI_POOL_CONTRACT_ID`
- [ ] Build production bundle: `npm run build`
- [ ] Test PWA functionality
- [ ] Verify service worker registration
- [ ] Test on mobile devices

## Deployment Steps

### 1. Database Migration
```bash
cd backend
sqlx migrate run --database-url $PRODUCTION_DATABASE_URL
```

### 2. Backend Deployment
```bash
# Stop current backend
sudo systemctl stop polypulse-backend

# Deploy new binary
cd backend
cargo build --release
sudo cp target/release/backend /usr/local/bin/polypulse-backend

# Restart service
sudo systemctl start polypulse-backend
sudo systemctl status polypulse-backend
```

### 3. Frontend Deployment
```bash
cd frontend
npm run build
rsync -avz dist/ /var/www/polypulse/
```

### 4. Nginx Configuration
```bash
# Reload nginx to pick up any config changes
sudo nginx -t
sudo systemctl reload nginx
```

## Post-Deployment Verification

### API Health Checks
- [ ] `GET /health` returns 200
- [ ] `GET /activities` returns data
- [ ] `GET /templates` returns 4 templates
- [ ] `GET /leaderboard/top_earners` returns data
- [ ] `POST /multi-pools` creates pool (authenticated)

### Smart Contract Verification
- [ ] Multi-pool contract is deployed
- [ ] Can create pool via contract
- [ ] Can join pool via contract
- [ ] Payout calculations are correct

### Frontend Verification
- [ ] Site loads correctly
- [ ] PWA install prompt appears
- [ ] Service worker is active
- [ ] WebSocket connection works
- [ ] New features are visible:
  - Activity feed
  - Bet templates
  - Leaderboard
  - Reputation scores

### Database Verification
```sql
-- Check new tables exist
SELECT COUNT(*) FROM activities;
SELECT COUNT(*) FROM bet_templates;
SELECT COUNT(*) FROM achievements;

-- Check default data
SELECT * FROM bet_templates;
SELECT * FROM achievements;

-- Check user columns
SELECT xp, level, reputation_score FROM users LIMIT 1;
```

## Monitoring

### Metrics to Watch
- [ ] API response times (<200ms p95)
- [ ] Database query performance
- [ ] Redis cache hit rate (>80%)
- [ ] Error rates (<1%)
- [ ] WebSocket connection count
- [ ] Multi-pool contract gas usage

### Logs to Monitor
- [ ] Backend application logs
- [ ] Nginx access/error logs
- [ ] PostgreSQL slow query log
- [ ] Redis logs

## Rollback Plan

If issues occur:

### 1. Rollback Backend
```bash
sudo systemctl stop polypulse-backend
sudo cp /usr/local/bin/polypulse-backend.backup /usr/local/bin/polypulse-backend
sudo systemctl start polypulse-backend
```

### 2. Rollback Frontend
```bash
rsync -avz /var/www/polypulse.backup/ /var/www/polypulse/
```

### 3. Rollback Database (if needed)
```bash
# Revert migrations
sqlx migrate revert --database-url $PRODUCTION_DATABASE_URL
```

## Performance Optimization

### After Deployment
- [ ] Monitor query performance
- [ ] Add indexes if needed:
  ```sql
  CREATE INDEX CONCURRENTLY idx_activities_timestamp ON activities(timestamp DESC);
  CREATE INDEX CONCURRENTLY idx_activities_bet_id ON activities(bet_id) WHERE bet_id IS NOT NULL;
  ```
- [ ] Enable Redis caching for:
  - Leaderboards (1 min TTL)
  - Bet templates (1 hour TTL)
  - Activity feed (30 sec TTL)
- [ ] Monitor memory usage
- [ ] Monitor CPU usage

## Security Checks

- [ ] All API endpoints require proper authentication
- [ ] Rate limiting is active
- [ ] CORS is properly configured
- [ ] SQL injection protection verified
- [ ] XSS protection headers set
- [ ] HTTPS enforced
- [ ] Secrets not exposed in logs

## Documentation Updates

- [ ] Update API documentation with new endpoints
- [ ] Update README.md with new features
- [ ] Update CHANGELOG.md
- [ ] Update deployment documentation

## Communication

- [ ] Notify team of deployment
- [ ] Announce new features to users
- [ ] Update status page
- [ ] Monitor user feedback

---

## Emergency Contacts

- **Backend Issues**: [Your contact]
- **Database Issues**: [DBA contact]
- **Frontend Issues**: [Frontend lead]
- **Infrastructure**: [DevOps contact]

---

*Last Updated: 2024-04-24*
