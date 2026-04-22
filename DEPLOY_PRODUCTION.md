# Production Deployment Guide

This guide covers deploying the Stellar-Only Platform to production.

## ⚠️ CRITICAL PRE-DEPLOYMENT REQUIREMENTS

- [ ] All staging tests passed (Task 9.2.7)
- [ ] Stakeholder approval obtained
- [ ] All Phase 7 tests completed (or documented exceptions)
- [ ] All Phase 8 documentation completed
- [ ] Security audit completed
- [ ] Backup strategy in place
- [ ] Rollback plan documented
- [ ] Monitoring and alerting configured
- [ ] SSL certificates obtained for production domains
- [ ] Production database and Redis instances ready
- [ ] Stellar mainnet contracts deployed and tested

## Prerequisites

- Production server with Docker and Docker Compose installed (4GB RAM minimum)
- Domain names configured:
  - `polypulse.co.ke` (frontend)
  - `api.polypulse.co.ke` (backend API)
- SSL certificates for both domains
- PostgreSQL database (production-grade)
- Redis instance (production-grade)
- Stellar mainnet account with XLM for contract deployment
- Access to production server via SSH
- Backup system configured

## Security Checklist (MUST COMPLETE BEFORE DEPLOYMENT)

- [ ] Generate strong JWT_SECRET: `openssl rand -base64 32`
- [ ] Generate strong POSTGRES_PASSWORD: `openssl rand -base64 32`
- [ ] Configure CORS_ALLOWED_ORIGINS to production domains only
- [ ] Enable HTTPS for all endpoints
- [ ] Configure firewall rules (allow only 80, 443, SSH)
- [ ] Set up fail2ban or similar intrusion prevention
- [ ] Configure Redis password protection
- [ ] Configure PostgreSQL SSL connections
- [ ] Enable rate limiting on backend
- [ ] Set up automated backups
- [ ] Configure log rotation
- [ ] Set up monitoring and alerting
- [ ] Review and update all secrets
- [ ] Disable debug logging
- [ ] Remove development dependencies

## Deployment Steps

### 1. Prepare Production Environment

```bash
# SSH into production server
ssh user@polypulse.co.ke

# Create application directory
sudo mkdir -p /opt/polypulse
sudo chown $USER:$USER /opt/polypulse
cd /opt/polypulse

# Clone repository
git clone https://github.com/your-org/polypulse.git .
git checkout main  # or specific release tag
```

### 2. Configure Environment Variables

```bash
# Copy production environment template
cp .env.production .env

# Edit with production values
nano .env
```

**CRITICAL: Update these values:**
- `JWT_SECRET` - Generate with: `openssl rand -base64 32`
- `POSTGRES_PASSWORD` - Use a strong password
- `CORS_ALLOWED_ORIGINS` - Set to production domains only
- `VITE_API_URL` - Set to https://api.polypulse.co.ke
- `VITE_WS_URL` - Set to wss://api.polypulse.co.ke
- `STELLAR_NETWORK` - Set to 'mainnet'
- `STELLAR_HORIZON_URL` - Set to https://horizon.stellar.org
- `STELLAR_RPC_URL` - Set to https://soroban-rpc.mainnet.stellar.org

### 3. Deploy Stellar Contracts to Mainnet

**⚠️ WARNING: This deploys to mainnet and costs XLM**

```bash
cd contracts

# Build contracts
stellar contract build

# Deploy market contract to mainnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market.wasm \
  --network mainnet \
  --source YOUR_MAINNET_SECRET_KEY

# IMPORTANT: Save the contract ID
# Update .env with:
# STELLAR_MARKET_CONTRACT_ID=<contract-id>
# VITE_STELLAR_MARKET_CONTRACT_ID=<contract-id>

# Deploy challenge contract to mainnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/challenge.wasm \
  --network mainnet \
  --source YOUR_MAINNET_SECRET_KEY

# IMPORTANT: Save the contract ID
# Update .env with:
# STELLAR_CHALLENGE_CONTRACT_ID=<contract-id>
# VITE_STELLAR_CHALLENGE_CONTRACT_ID=<contract-id>

# Test contracts on mainnet
stellar contract invoke \
  --id <market-contract-id> \
  --source YOUR_MAINNET_SECRET_KEY \
  --network mainnet \
  -- \
  get_version
```

### 4. Set Up Database

```bash
# Start database service
docker compose up -d db

# Wait for database to be ready
docker compose exec db pg_isready -U polypulse

# Run migrations
docker compose exec backend sqlx migrate run

# Create database backup
docker compose exec db pg_dump -U polypulse polypulse > backup_pre_deploy.sql
```

### 5. Build and Deploy Services

```bash
# Build services
docker compose build --no-cache

# Start all services
docker compose up -d

# Check service status
docker compose ps

# Check logs
docker compose logs -f
```

### 6. Configure Reverse Proxy (Nginx)

```bash
# Install Nginx (if not already installed)
sudo apt update
sudo apt install nginx certbot python3-certbot-nginx

# Create Nginx configuration
sudo nano /etc/nginx/sites-available/polypulse.co.ke
```

**Nginx Configuration:**

```nginx
# Frontend - polypulse.co.ke
server {
    listen 80;
    server_name polypulse.co.ke www.polypulse.co.ke;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name polypulse.co.ke www.polypulse.co.ke;

    ssl_certificate /etc/letsencrypt/live/polypulse.co.ke/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/polypulse.co.ke/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    location / {
        proxy_pass http://localhost:5173;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# Backend API - api.polypulse.co.ke
server {
    listen 80;
    server_name api.polypulse.co.ke;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.polypulse.co.ke;

    ssl_certificate /etc/letsencrypt/live/api.polypulse.co.ke/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.polypulse.co.ke/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
    limit_req zone=api_limit burst=20 nodelay;

    location / {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_read_timeout 86400;
    }

    # Health check endpoint (no rate limiting)
    location /health {
        proxy_pass http://localhost:8000/health;
        access_log off;
    }
}
```

**Enable site and obtain SSL certificates:**

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/polypulse.co.ke /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Obtain SSL certificates
sudo certbot --nginx -d polypulse.co.ke -d www.polypulse.co.ke
sudo certbot --nginx -d api.polypulse.co.ke

# Reload Nginx
sudo systemctl reload nginx

# Enable auto-renewal
sudo systemctl enable certbot.timer
```

### 7. Configure Firewall

```bash
# Configure UFW firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# Verify firewall status
sudo ufw status
```

### 8. Set Up Monitoring

```bash
# Install monitoring tools
sudo apt install prometheus-node-exporter

# Configure log rotation
sudo nano /etc/logrotate.d/polypulse
```

**Log rotation configuration:**

```
/opt/polypulse/logs/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 www-data www-data
    sharedscripts
    postrotate
        docker compose restart backend
    endscript
}
```

### 9. Set Up Automated Backups

```bash
# Create backup script
sudo nano /opt/polypulse/backup.sh
```

**Backup script:**

```bash
#!/bin/bash
BACKUP_DIR="/opt/polypulse/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup database
docker compose exec -T db pg_dump -U polypulse polypulse > $BACKUP_DIR/db_$DATE.sql

# Compress backup
gzip $BACKUP_DIR/db_$DATE.sql

# Keep only last 30 days of backups
find $BACKUP_DIR -name "db_*.sql.gz" -mtime +30 -delete

echo "Backup completed: db_$DATE.sql.gz"
```

```bash
# Make script executable
sudo chmod +x /opt/polypulse/backup.sh

# Add to crontab (daily at 2 AM)
sudo crontab -e
# Add line:
# 0 2 * * * /opt/polypulse/backup.sh >> /var/log/polypulse_backup.log 2>&1
```

## Post-Deployment Verification (Task 9.3)

### 1. Health Checks

```bash
# Backend health
curl https://api.polypulse.co.ke/health

# Frontend access
curl -I https://polypulse.co.ke

# Database connection
docker compose exec db psql -U polypulse -d polypulse -c "SELECT 1;"

# Redis connection
docker compose exec redis redis-cli ping
```

### 2. Wallet Connection Test (Task 9.3.1)

1. Visit https://polypulse.co.ke
2. Click "Connect Wallet"
3. Connect Freighter wallet
4. Verify wallet address displays
5. Verify balance displays

### 3. Authentication Flow Test (Task 9.3.2)

1. Connect wallet from login page
2. Sign authentication message
3. Verify redirect to /social-login
4. Verify JWT tokens stored
5. Test authenticated API calls

### 4. Protected Routes Test (Task 9.3.3)

1. Navigate to protected routes while connected
2. Verify access granted
3. Disconnect wallet
4. Verify redirect to /login
5. Reconnect and verify access restored

### 5. Monitor Logs (Task 9.3.4)

```bash
# Watch backend logs
docker compose logs -f backend

# Watch frontend logs
docker compose logs -f frontend

# Watch Nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

### 6. Performance Metrics (Task 9.3.5)

```bash
# Check response times
curl -w "@curl-format.txt" -o /dev/null -s https://polypulse.co.ke

# Monitor resource usage
docker stats

# Check database performance
docker compose exec db psql -U polypulse -d polypulse -c "SELECT * FROM pg_stat_activity;"
```

## Monitoring and Alerting (Phase 10)

### Set Up Error Tracking (Task 10.2.1)

Consider integrating:
- Sentry for error tracking
- Prometheus for metrics
- Grafana for visualization
- Uptime monitoring (UptimeRobot, Pingdom)

### Key Metrics to Monitor (Tasks 10.1.2-10.1.5)

- Wallet connection success rate
- Authentication completion rate
- Average connection time
- API response times
- Error rates
- Server resource usage (CPU, memory, disk)
- Database connection pool usage

### Set Up Alerts

Configure alerts for:
- Service downtime
- High error rates (>5%)
- Slow response times (>2s)
- High CPU/memory usage (>80%)
- Database connection failures
- SSL certificate expiration (30 days before)

## Rollback Procedure

If critical issues are found:

```bash
# 1. Stop services
docker compose down

# 2. Restore database backup
docker compose up -d db
docker compose exec -T db psql -U polypulse polypulse < backup_pre_deploy.sql

# 3. Checkout previous version
git checkout <previous-release-tag>

# 4. Rebuild and restart
docker compose up -d --build

# 5. Verify rollback
curl https://api.polypulse.co.ke/health
```

## Post-Launch Tasks

### Immediate (First 24 hours)

- [ ] Monitor error logs continuously
- [ ] Monitor user feedback
- [ ] Check wallet connection success rate
- [ ] Verify all critical paths work
- [ ] Monitor server resources

### Short-term (First week)

- [ ] Collect user feedback (Task 10.3.1)
- [ ] Monitor support tickets (Task 10.3.2)
- [ ] Analyze performance metrics
- [ ] Address any issues found
- [ ] Optimize based on real usage patterns

### Long-term (Ongoing)

- [ ] Conduct user satisfaction surveys (Task 10.3.3)
- [ ] Iterate on UX based on feedback (Task 10.3.4)
- [ ] Optimize bundle size if needed (Task 10.4.1)
- [ ] Optimize API calls if needed (Task 10.4.2)
- [ ] Keep dependencies updated
- [ ] Regular security audits

## Common Production Issues

### Issue: High latency
**Solution**: Check database queries, add caching, optimize API calls

### Issue: Wallet connection failures
**Solution**: Verify Stellar mainnet is accessible, check Horizon API status

### Issue: Authentication failures
**Solution**: Check JWT_SECRET, verify backend logs, check token expiration

### Issue: CORS errors
**Solution**: Verify CORS_ALLOWED_ORIGINS includes production domains

### Issue: SSL certificate errors
**Solution**: Renew certificates with certbot, check Nginx configuration

## Maintenance

### Regular Tasks

- **Daily**: Check logs for errors
- **Weekly**: Review performance metrics, check backups
- **Monthly**: Update dependencies, security patches, review user feedback
- **Quarterly**: Security audit, performance optimization

### Updating the Application

```bash
# 1. Backup current state
/opt/polypulse/backup.sh

# 2. Pull latest changes
git pull origin main

# 3. Rebuild services
docker compose build --no-cache

# 4. Run migrations
docker compose exec backend sqlx migrate run

# 5. Restart services
docker compose up -d

# 6. Verify deployment
curl https://api.polypulse.co.ke/health
```

## Support and Documentation

- Production URL: https://polypulse.co.ke
- API URL: https://api.polypulse.co.ke
- Documentation: README.md, DEPLOY.md
- Specs: `.kiro/specs/stellar-only-platform/`
- Issues: GitHub Issues
- Monitoring: Configure your monitoring dashboard URL

## Emergency Contacts

- DevOps Lead: [contact info]
- Backend Lead: [contact info]
- Frontend Lead: [contact info]
- Security Lead: [contact info]

---

**Remember**: Production deployment is irreversible. Ensure all checks pass before proceeding.
