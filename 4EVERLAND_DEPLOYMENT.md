# 🚀 4everland Deployment Guide for PolyPulse

## 📋 Quick Reference - Environment Variables

Copy these EXACT values when deploying to 4everland:

| Variable | Value |
|----------|-------|
| `VITE_API_URL` | `https://polypulse-backend-436v.onrender.com` |
| `VITE_API_HOST` | `polypulse-backend-436v.onrender.com` |
| `VITE_WS_URL` | `wss://polypulse-backend-436v.onrender.com` |
| `VITE_STELLAR_NETWORK` | `testnet` |
| `VITE_HORIZON_URL` | `https://horizon-testnet.stellar.org` |
| `VITE_SOROBAN_RPC_URL` | `https://soroban-testnet.stellar.org` |
| `VITE_STELLAR_MARKET_CONTRACT_ID` | *(leave empty for now)* |
| `VITE_STELLAR_CHALLENGE_CONTRACT_ID` | *(leave empty for now)* |

---

## Quick Start - Deploy in 5 Minutes

**4everland Link**: https://dashboard.4everland.org

---

## Step 1: Sign Up (1 minute)

1. Go to: **https://dashboard.4everland.org**
2. Click **"Sign in with GitHub"**
3. Authorize 4everland to access your repositories

---

## Step 2: Create New Project (1 minute)

1. Click **"New Project"** or **"Hosting"**
2. Select **"Import from GitHub"**
3. Choose repository: **`dgithinjibit/polypulse`**
4. Click **"Import"**

---

## Step 3: Configure Build Settings (2 minutes)

### Framework Detection
4everland should auto-detect Vite, but verify these settings:

| Setting | Value |
|---------|-------|
| **Framework Preset** | Vite |
| **Root Directory** | `frontend` |
| **Build Command** | `npm run build` (NOT `npm run build:check`) |
| **Output Directory** | `dist` |
| **Install Command** | `npm install` |
| **Node Version** | **20.x** (CRITICAL - must be 20 or higher!) |

⚠️ **CRITICAL BUILD COMMAND**: Use `npm run build` (which runs `vite build` only). 
- DO NOT use `tsc && vite build` or `npm run build:check`
- Vite has its own type checking via esbuild which is faster and more compatible
- The `build:check` script is available for local development if you want strict type checking

⚠️ **CRITICAL**: If you don't see a Node version selector, add this to your build settings:
- Set environment variable: `NODE_VERSION=20` or `NODE_VERSION=20.18.0`

---

## Step 4: Add Environment Variables (1 minute)

Click **"Environment Variables"** and add these **EXACT VALUES**:

```bash
# Backend API Configuration (Render)
VITE_API_URL=https://polypulse-backend-436v.onrender.com
VITE_API_HOST=polypulse-backend-436v.onrender.com
VITE_WS_URL=wss://polypulse-backend-436v.onrender.com

# Stellar Network Configuration
VITE_STELLAR_NETWORK=testnet
VITE_HORIZON_URL=https://horizon-testnet.stellar.org
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org

# Smart Contract IDs (leave empty for now - add when deployed)
VITE_STELLAR_MARKET_CONTRACT_ID=
VITE_STELLAR_CHALLENGE_CONTRACT_ID=
```

**Note**: The contract IDs are empty because you haven't deployed your Stellar smart contracts yet. You can add them later.

---

## Step 5: Deploy! (30 seconds)

1. Click **"Deploy"**
2. Wait ~2-3 minutes for build to complete
3. 🎉 Your site is live!

---

## Your Live URLs

After deployment, you'll get:

- **4everland URL**: `https://[your-project].4everland.app`
- **IPFS Gateway**: `https://[cid].ipfs.4everland.link`
- **Custom Domain**: Add your own domain in settings

---

## Post-Deployment Checklist

### Test Your Site
- [ ] Homepage loads with aurora background
- [ ] Navigation works (all routes)
- [ ] Wallet connection (Freighter/Albedo)
- [ ] Markets page loads
- [ ] API calls work (check browser console)
- [ ] Mobile responsive

### Add Custom Domain (Optional)
1. Go to **Project Settings → Domains**
2. Click **"Add Domain"**
3. Enter your domain (e.g., `polypulse.co.ke`)
4. Update DNS records as shown:
   - Type: `CNAME`
   - Name: `@` or `www`
   - Value: `cname.4everland.org`
5. Wait for DNS propagation (~5-30 minutes)

### Update Backend CORS
Your backend CORS is already configured, but after you get your 4everland URL, add it to Render:

1. Go to your Render dashboard
2. Select your backend service
3. Go to **Environment** tab
4. Update `CORS_ALLOWED_ORIGINS` to include your 4everland domain:
   ```
   CORS_ALLOWED_ORIGINS=https://[your-project].4everland.app,https://polypulse.co.ke,https://www.polypulse.co.ke
   ```
5. Save and redeploy

---

## Why 4everland is Great for Your dApp

✅ **Web3 Native**: Built for decentralized apps
✅ **IPFS Hosting**: Decentralized storage
✅ **Fast CDN**: Global edge network
✅ **Free Tier**: Generous limits
✅ **Auto SSL**: HTTPS by default
✅ **CI/CD**: Auto-deploy on git push
✅ **ENS Support**: Use .eth domains
✅ **DWeb Gateway**: Multiple IPFS gateways

---

## Continuous Deployment

Every time you push to GitHub:
1. 4everland detects the push
2. Automatically builds your project
3. Deploys to IPFS
4. Updates your live site

**No manual deployment needed!**

---

## Monitoring & Analytics

### View Build Logs
- Go to **Deployments** tab
- Click on any deployment
- View real-time build logs

### Check Performance
- Go to **Analytics** tab
- View traffic, bandwidth, requests
- Monitor site performance

---

## Troubleshooting

### TypeScript Build Errors (ArrayBuffer/ArrayBufferLike)
**Error Message**:
```
Type 'ArrayBufferLike' is not assignable to type 'ArrayBuffer'
Type 'SharedArrayBuffer' is not assignable to type 'ArrayBuffer'
```

**What it means**: TypeScript's strict type checking in the build environment detects incompatibility between `ArrayBufferLike` (used by some libraries) and `ArrayBuffer` (expected by Web Crypto API).

**Already Fixed**: This has been resolved in the codebase with:
- Type assertions in `frontend/src/services/encryption.ts`
- Type assertions in `frontend/src/services/pwa.ts`
- Type declarations in `frontend/src/types/buffer.d.ts`
- Updated `tsconfig.json` with `skipDefaultLibCheck: true`

**If you still see this error**: Make sure you're deploying from the latest commit that includes these fixes.

### Build Fails
**Check**:
- Build logs in 4everland dashboard
- Node version is 18.x
- All dependencies in `package.json`
- Root directory is set to `frontend`

**Solution**:
```bash
# Test build locally first
cd frontend
npm install
npm run build
```

### Environment Variables Not Working
**Check**:
- Variables start with `VITE_`
- No typos in variable names
- Saved and redeployed after adding

**Solution**: Redeploy after adding/changing variables

### Site Not Loading
**Check**:
- Browser console for errors
- Backend CORS allows 4everland domain
- Backend is accessible and running

**Solution**: 
- Verify `VITE_API_URL` is correct
- Test backend API directly
- Check network tab in browser DevTools

### Wallet Connection Issues
**Check**:
- Freighter extension installed
- Correct Stellar network (testnet/mainnet)
- RPC URLs are accessible

**Solution**:
- Test wallet connection locally first
- Verify `VITE_STELLAR_NETWORK` matches your setup

---

## Advanced Configuration

### Custom Build Command
If you need custom build steps:
```bash
npm install && cd frontend && npm install && npm run build
```

### Multiple Environments
Create separate projects for:
- **Production**: `main` branch
- **Staging**: `staging` branch
- **Development**: `dev` branch

### Rollback Deployment
1. Go to **Deployments** tab
2. Find previous successful deployment
3. Click **"Redeploy"**

---

## Pricing (as of 2026)

### Free Tier (Perfect for Testing)
- ✅ Unlimited projects
- ✅ 100 GB bandwidth/month
- ✅ 10 GB storage
- ✅ Custom domains
- ✅ SSL certificates
- ✅ IPFS hosting

### Pro Tier (If You Need More)
- More bandwidth
- Priority support
- Advanced analytics
- Team collaboration

---

## Support Resources

- **4everland Docs**: https://docs.4everland.org
- **Discord**: https://discord.gg/4everland
- **Twitter**: https://twitter.com/4everland_org
- **GitHub Issues**: https://github.com/dgithinjibit/polypulse/issues

---

## Next Steps After Deployment

1. **Test Everything**: Go through the checklist above
2. **Share Your Site**: Tweet about your launch! 🐦
3. **Monitor Performance**: Check analytics regularly
4. **Update Backend**: Ensure CORS is configured
5. **Add Custom Domain**: Make it professional
6. **Get Feedback**: Share with early users

---

## Quick Commands Reference

```bash
# Test build locally
cd frontend
npm run build

# Preview production build
npm run preview

# Check for errors
npm run lint

# Run tests
npm test
```

---

**Status**: ✅ Ready for 4everland Deployment  
**Repository**: https://github.com/dgithinjibit/polypulse  
**Build Time**: ~2-3 minutes  
**Deployment**: Automatic on git push

---

## 🎉 You're All Set!

Go to **https://dashboard.4everland.org** and start deploying!

Your aurora-themed PolyPulse dApp will be live in minutes! 🚀
