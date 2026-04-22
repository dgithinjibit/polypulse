# Fleek Deployment Guide for PolyPulse

## Pre-Deployment Checklist ✅

### 1. Build Status
- ✅ Frontend builds successfully
- ✅ No TypeScript errors
- ✅ No console.log statements in production code
- ✅ All glassmorphism styling applied

### 2. Environment Variables Required

Set these in Fleek dashboard before deploying:

```bash
# Backend API Configuration
VITE_API_URL=https://your-backend-domain.com
VITE_API_HOST=your-backend-domain.com
VITE_WS_URL=wss://your-backend-domain.com

# Stellar Network Configuration
VITE_STELLAR_NETWORK=testnet
VITE_HORIZON_URL=https://horizon-testnet.stellar.org
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org

# Smart Contract IDs (if deployed)
VITE_STELLAR_MARKET_CONTRACT_ID=your-market-contract-id
VITE_STELLAR_CHALLENGE_CONTRACT_ID=your-challenge-contract-id
```

### 3. Fleek Configuration

**Build Settings:**
- Framework: Vite
- Build Command: `npm run build`
- Publish Directory: `dist`
- Base Directory: `frontend`
- Node Version: 18.x or higher

### 4. Deployment Steps

1. **Connect Repository to Fleek**
   - Go to https://app.fleek.co
   - Click "Add new site"
   - Connect your GitHub/GitLab repository
   - Select the branch to deploy (e.g., `main`)

2. **Configure Build Settings**
   - Set base directory to `frontend`
   - Set build command to `npm run build`
   - Set publish directory to `dist`

3. **Add Environment Variables**
   - Go to Site Settings → Environment Variables
   - Add all variables listed above
   - Make sure to use your actual backend URL

4. **Deploy**
   - Click "Deploy site"
   - Wait for build to complete (~2-3 minutes)
   - Your site will be live at `https://your-site.on.fleek.co`

### 5. Post-Deployment Verification

Test these features after deployment:

- [ ] Homepage loads with aurora background
- [ ] Glassmorphism cards render correctly
- [ ] Navigation works (all routes)
- [ ] Wallet connection (Freighter/Albedo)
- [ ] Markets page loads data
- [ ] WebSocket connections work
- [ ] Responsive design on mobile

### 6. Known Optimizations

**Bundle Size:**
- Main bundle: ~509 KB (gzipped: ~141 KB)
- Stellar SDK: ~1 MB (gzipped: ~278 KB)
- Consider code-splitting for further optimization

**Performance Tips:**
- Glassmorphism effects use `backdrop-filter` - ensure browser support
- Aurora orbs use CSS animations - minimal performance impact
- WebSocket connections auto-reconnect on failure

### 7. Troubleshooting

**Issue: White screen after deployment**
- Check browser console for errors
- Verify environment variables are set correctly
- Ensure backend CORS allows your Fleek domain

**Issue: Wallet connection fails**
- Verify Stellar network configuration
- Check that Freighter extension is installed
- Ensure RPC URLs are accessible

**Issue: API calls fail**
- Verify VITE_API_URL is set correctly
- Check backend CORS configuration
- Ensure backend is deployed and accessible

**Issue: Glassmorphism not showing**
- Check browser compatibility (Safari needs `-webkit-backdrop-filter`)
- Verify CSS is loading correctly
- Clear browser cache

### 8. Custom Domain (Optional)

To use a custom domain:
1. Go to Site Settings → Domain Management
2. Add your custom domain
3. Update DNS records as instructed
4. Update backend CORS to allow your custom domain

### 9. Continuous Deployment

Fleek automatically deploys when you push to your connected branch:
- Push to `main` → Auto-deploy to production
- Create preview deployments for PRs
- Rollback to previous deployments if needed

### 10. Backend Requirements

Ensure your backend is configured to accept requests from Fleek:

```rust
// In backend CORS configuration
CORS_ALLOWED_ORIGINS=https://your-site.on.fleek.co,https://your-custom-domain.com
```

---

## Quick Deploy Command

If you prefer CLI deployment:

```bash
# Install Fleek CLI
npm install -g @fleek-platform/cli

# Login
fleek login

# Deploy
cd frontend
fleek sites deploy
```

---

## Support

- Fleek Docs: https://docs.fleek.co
- Fleek Discord: https://discord.gg/fleek
- PolyPulse Issues: [Your GitHub Issues URL]

---

**Last Updated:** April 22, 2026
**Build Status:** ✅ Ready for Production
**Aurora Theme:** ✅ Applied
