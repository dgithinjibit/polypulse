# 🚀 Redeployment Guide for v2.0

## Your Current Setup

- **Frontend**: 4everland (auto-deploys from GitHub ✅)
- **Backend**: Render (needs configuration)
- **Database**: PostgreSQL (Neon/Supabase/Railway - you need to identify which one)

---

## 🎯 Quick Redeployment Steps

### Step 1: Identify Your Database Service

Check which PostgreSQL service you're using:

1. **Check your email** for signup confirmations from:
   - Neon (neon.tech)
   - Supabase (supabase.com)
   - Railway (railway.app)
   - ElephantSQL
   - Aiven

2. **Check your browser bookmarks** for database dashboards

3. **Check your .env file** - the DATABASE_URL will have a clue:
   - Neon: `postgres://...@ep-...neon.tech/...`
   - Supabase: `postgres://...@db....supabase.co/...`
   - Railway: `postgres://...@...railway.app/...`

---

### Step 2: Run Database Migrations

Once you identify your database service:

```bash
# Option A: Using your local .env file
cd backend
source ../.env  # or export DATABASE_URL=your-url
sqlx migrate run

# Option B: Direct command
cd backend
sqlx migrate run --database-url "postgresql://user:pass@host:5432/dbname"
```

**What this does**:
- Creates 7 new tables
- Adds 8 new columns to users table
- Adds 4 new columns to p2p_bets table
- Inserts 5 default achievements
- Inserts 4 default bet templates

---

### Step 3: Configure Render Auto-Deploy

#### Option A: Enable Auto-Deploy (Recommended)

1. Go to https://dashboard.render.com
2. Select your backend service
3. Go to **Settings** tab
4. Scroll to **Build & Deploy**
5. Enable **Auto-Deploy**: Choose **"After CI checks pass"**
6. Click **Save Changes**

#### Option B: Set Up Deploy Hook for GitHub Actions

1. In Render dashboard, go to **Settings**
2. Scroll to **Deploy Hook**
3. Click **Create Deploy Hook**
4. Copy the webhook URL
5. In GitHub, go to your repo → **Settings** → **Secrets and variables** → **Actions**
6. Click **New repository secret**
7. Name: `RENDER_DEPLOY_HOOK`
8. Value: Paste the webhook URL
9. Click **Add secret**

Now GitHub Actions will automatically trigger Render deployment after tests pass!

---

### Step 4: Update Environment Variables

#### On Render:

1. Go to Render dashboard
2. Select your backend service
3. Go to **Environment** tab
4. Add new variable:
   ```
   STELLAR_MULTI_POOL_CONTRACT_ID=<will-add-after-contract-deployment>
   ```
5. Click **Save Changes**

#### On 4everland:

1. Go to https://dashboard.4everland.org
2. Select your project
3. Go to **Settings** → **Environment Variables**
4. Add new variable:
   ```
   VITE_STELLAR_MULTI_POOL_CONTRACT_ID=<will-add-after-contract-deployment>
   ```
5. Click **Save**
6. **Redeploy** the project

---

### Step 5: Deploy Smart Contracts

```bash
cd contracts

# Build all contracts
stellar contract build

# Deploy multi-pool contract to testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm \
  --network testnet \
  --source <your-stellar-secret-key>

# Save the contract ID that's returned
# Example: CBQHNAXSI55GX2GN6D67GK7BHVPSLJUGZQEU7WJ5LKR5PNUCGLIMAO4K
```

**Then update environment variables** with the contract ID (repeat Step 4).

---

### Step 6: Verify Deployment

#### Check Backend (Render):

```bash
# Test health endpoint
curl https://polypulse-backend-436v.onrender.com/health

# Test new endpoints
curl https://polypulse-backend-436v.onrender.com/api/v1/activities
curl https://polypulse-backend-436v.onrender.com/api/v1/templates
curl https://polypulse-backend-436v.onrender.com/api/v1/leaderboard/top_earners
```

#### Check Frontend (4everland):

1. Visit your 4everland URL
2. Open browser console (F12)
3. Check for errors
4. Test new features:
   - Activity feed
   - Bet templates
   - Leaderboard
   - Reputation scores

#### Check Database:

```bash
# Connect to your database
psql $DATABASE_URL

# Verify new tables
\dt

# Check activities table
SELECT COUNT(*) FROM activities;

# Check bet_templates
SELECT * FROM bet_templates;

# Check achievements
SELECT * FROM achievements;
```

---

## 🔄 Auto-Deploy Flow (After Setup)

Once configured, your deployment flow will be:

```
1. Push to GitHub
   ↓
2. GitHub Actions runs tests
   ↓
3. If tests pass:
   - Triggers Render deployment (backend)
   - 4everland auto-deploys (frontend)
   ↓
4. Both services update automatically
   ↓
5. ✅ v2.0 features live!
```

---

## 🐛 Troubleshooting

### Database Migration Fails

**Error**: `relation "activities" already exists`
- **Solution**: Migrations already ran. Skip to next step.

**Error**: `connection refused`
- **Solution**: Check DATABASE_URL is correct
- Verify database service is running
- Check firewall/IP whitelist

### Render Deployment Fails

**Check**:
1. Render logs: Dashboard → Logs tab
2. Build command is correct: `cargo build --release`
3. Start command is correct: `./target/release/backend`
4. Environment variables are set

### 4everland Not Updating

**Check**:
1. GitHub push was successful
2. 4everland detected the push (check Deployments tab)
3. Build logs for errors
4. Environment variables include `VITE_` prefix

### Smart Contract Deployment Fails

**Check**:
1. Stellar CLI is installed: `stellar --version`
2. You have testnet XLM for fees
3. Network is set to testnet
4. Contract builds successfully: `stellar contract build`

---

## 📊 Deployment Checklist

- [ ] Identified database service
- [ ] Ran database migrations
- [ ] Verified new tables exist
- [ ] Configured Render auto-deploy
- [ ] Added RENDER_DEPLOY_HOOK to GitHub secrets
- [ ] Updated Render environment variables
- [ ] Updated 4everland environment variables
- [ ] Deployed multi-pool smart contract
- [ ] Updated contract IDs in environment variables
- [ ] Tested backend health endpoint
- [ ] Tested new API endpoints
- [ ] Tested frontend loads correctly
- [ ] Verified new features work
- [ ] Checked for console errors
- [ ] Tested on mobile

---

## 🎉 Success Indicators

You'll know deployment succeeded when:

✅ Backend health check returns 200
✅ New API endpoints return data
✅ Frontend loads without errors
✅ Activity feed shows data
✅ Bet templates are available
✅ Leaderboard displays
✅ Reputation scores visible
✅ Multi-pool contract is callable

---

## 📞 Need Help?

### Common Database Services:

**Neon**: https://console.neon.tech
**Supabase**: https://app.supabase.com
**Railway**: https://railway.app
**Render PostgreSQL**: https://dashboard.render.com

### Deployment Services:

**Render**: https://dashboard.render.com
**4everland**: https://dashboard.4everland.org
**GitHub Actions**: https://github.com/dgithinjibit/polypulse/actions

---

## 🚀 Quick Commands Reference

```bash
# Database migration
cd backend && sqlx migrate run

# Build contracts
cd contracts && stellar contract build

# Deploy contract
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm --network testnet

# Test backend locally
cd backend && cargo run

# Test frontend locally
cd frontend && npm run dev

# Run all tests
cd backend && cargo test
cd backend && cargo test --test multi_pool_payout_tests

# Check deployment status
curl https://polypulse-backend-436v.onrender.com/health
```

---

*Last Updated: 2024-04-24*
*Version: 2.0.0*
