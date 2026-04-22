# 🚀 Quick Start: Deploy to Fleek in 5 Minutes

## Prerequisites
- ✅ Code is ready (you're here!)
- ✅ GitHub repository with your code
- ✅ Backend deployed (or URL ready)
- ✅ Fleek account (sign up at https://app.fleek.co)

---

## Step 1: Go to Fleek (30 seconds)
1. Open https://app.fleek.co
2. Sign in with GitHub
3. Click **"Add new site"**

---

## Step 2: Connect Repository (1 minute)
1. Select your GitHub repository
2. Choose branch: `main` (or your deployment branch)
3. Click **"Continue"**

---

## Step 3: Configure Build (1 minute)
Set these values:

| Setting | Value |
|---------|-------|
| **Framework** | Vite |
| **Base Directory** | `frontend` |
| **Build Command** | `npm run build` |
| **Publish Directory** | `dist` |
| **Node Version** | `18.x` |

Click **"Continue"**

---

## Step 4: Add Environment Variables (2 minutes)

Click **"Add Environment Variable"** and add these:

### Required Variables
```bash
VITE_API_URL=https://your-backend-url.com
VITE_API_HOST=your-backend-url.com
VITE_WS_URL=wss://your-backend-url.com
VITE_STELLAR_NETWORK=testnet
VITE_HORIZON_URL=https://horizon-testnet.stellar.org
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
```

### Optional (if you have contracts deployed)
```bash
VITE_STELLAR_MARKET_CONTRACT_ID=your-contract-id
VITE_STELLAR_CHALLENGE_CONTRACT_ID=your-contract-id
```

**Important**: Replace `your-backend-url.com` with your actual backend URL!

---

## Step 5: Deploy! (30 seconds)
1. Click **"Deploy site"**
2. Wait for build to complete (~2-3 minutes)
3. 🎉 Your site is live!

---

## Step 6: Test Your Site (1 minute)

Visit your Fleek URL: `https://[your-site].on.fleek.co`

Quick checks:
- [ ] Homepage loads with aurora background
- [ ] Click "Markets" - does it load?
- [ ] Try connecting wallet
- [ ] Check mobile view

---

## 🎉 Done!

Your PolyPulse dApp is now live on Fleek!

### What's Next?

**Add Custom Domain** (optional):
1. Go to Site Settings → Domain Management
2. Add your domain (e.g., `polypulse.co.ke`)
3. Update DNS records as shown
4. Update backend CORS to allow your domain

**Monitor Your Site**:
- Check Fleek dashboard for build logs
- Monitor analytics (if configured)
- Watch for errors in browser console

**Share with Users**:
- Tweet your launch 🐦
- Share on Discord/Telegram
- Get feedback from early users

---

## 🆘 Something Wrong?

### Build Failed?
- Check build logs in Fleek dashboard
- Verify environment variables are set
- Ensure Node version is 18.x

### Site Not Loading?
- Check browser console for errors
- Verify backend URL is correct
- Check backend CORS configuration

### Need Help?
- Read: `FLEEK_DEPLOYMENT.md` (detailed guide)
- Check: `PRE_DEPLOYMENT_CHECKLIST.md`
- Fleek Docs: https://docs.fleek.co
- Fleek Discord: https://discord.gg/fleek

---

**Total Time**: ~5 minutes  
**Difficulty**: Easy  
**Status**: ✅ Ready to deploy!
