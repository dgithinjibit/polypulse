# CORS and CSP Fixes Documentation

## Problem Summary

When deploying the PolyPulse frontend to 4everland IPFS, we encountered two critical browser security issues:

1. **CORS Error**: Backend blocked requests from the IPFS-hosted frontend
2. **CSP Violation**: Content Security Policy blocked `eval()` usage in JavaScript

---

## Issue 1: CORS (Cross-Origin Resource Sharing)

### The Problem

```
Access to XMLHttpRequest at 'https://polypulse-backend-436v.onrender.com/api/v1/polls?status=open' 
from origin 'https://polypulse-nfphmvqb-dgithinjibit.ipfs.4everland.app' 
has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header is present on the requested resource.
```

### Root Cause

4everland generates a **new hash for every deployment**, changing the URL:
- Old: `polypulse-bfmrrbtj-dgithinjibit.ipfs.4everland.app`
- New: `polypulse-nfphmvqb-dgithinjibit.ipfs.4everland.app`

The backend's `CORS_ORIGINS` env var had the old URL hardcoded, so the browser blocked the new origin.

### Why This Happens

CORS is a browser security mechanism that prevents websites from making requests to different domains unless explicitly allowed. The backend must send an `Access-Control-Allow-Origin` header matching the requesting origin.

### The Solution

Instead of updating `CORS_ORIGINS` on every deployment, we modified the backend to accept **any** `*.ipfs.4everland.app` subdomain using a predicate-based origin check.

**File**: `backend/src/routes/mod.rs`

**Before**:
```rust
CorsLayer::new()
    .allow_origin(AllowOrigin::list(origins))
    .allow_methods(Any)
    .allow_headers(Any)
```

**After**:
```rust
let origins_clone = origins.clone();
CorsLayer::new()
    .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
        // Allow any exact match from the configured list
        if origins_clone.contains(origin) {
            return true;
        }
        // Allow any 4everland IPFS deployment subdomain
        origin
            .to_str()
            .map(|s| s.ends_with(".ipfs.4everland.app"))
            .unwrap_or(false)
    }))
    .allow_methods(Any)
    .allow_headers(Any)
```

### What This Does

1. Checks if the origin is in the explicit `CORS_ORIGINS` list (e.g., `http://localhost:5173`)
2. If not, checks if it ends with `.ipfs.4everland.app`
3. Allows the request if either condition is true

### Benefits

- No need to update backend config on every frontend deployment
- Still maintains security by only allowing 4everland IPFS domains
- Localhost and other configured origins still work

---

## Issue 2: CSP (Content Security Policy)

### The Problem

```
Content Security Policy of your site blocks the use of 'eval' in JavaScript
```

Browser console showed the app was trying to use `eval()`, which was blocked by a strict CSP.

### Root Cause

Two sources of CSP restrictions:

1. **4everland Gateway**: IPFS gateways inject strict security headers
2. **Node.js Polyfills**: The `vite-plugin-node-polyfills` package (needed for Stellar SDK) includes a `vm` module polyfill that uses `eval()` internally

### Why This Happens

CSP is a security layer that prevents XSS attacks by blocking dangerous JavaScript patterns like `eval()`. While this is good for security, some legitimate libraries need these features.

### The Solution

We made two changes:

#### 1. Exclude the `vm` Polyfill

**File**: `frontend/vite.config.ts`

```typescript
nodePolyfills({
  globals: { global: true, process: true, Buffer: true },
  protocolImports: true,
  // Exclude eval-based polyfills to comply with strict CSP on IPFS gateways
  exclude: ['vm'],
}),
```

The `vm` module is a Node.js feature for running code in sandboxes. We don't need it in the browser, so we exclude it.

#### 2. Add Explicit CSP Meta Tag

**File**: `frontend/index.html`

```html
<meta http-equiv="Content-Security-Policy" content="
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  connect-src 'self' https://polypulse-backend-436v.onrender.com wss://polypulse-backend-436v.onrender.com https://horizon-testnet.stellar.org https://soroban-testnet.stellar.org;
  img-src 'self' data: blob:;
  font-src 'self' data:;
  worker-src 'self' blob:;
" />
```

### CSP Directives Explained

| Directive | Value | Purpose |
|-----------|-------|---------|
| `default-src` | `'self'` | Only load resources from same origin by default |
| `script-src` | `'self' 'wasm-unsafe-eval'` | Allow scripts from same origin + WebAssembly (Stellar SDK needs this) |
| `style-src` | `'self' 'unsafe-inline'` | Allow inline styles (React uses these) |
| `connect-src` | Multiple origins | Allow API calls to backend, Stellar network |
| `img-src` | `'self' data: blob:` | Allow images from same origin, data URLs, blobs |
| `font-src` | `'self' data:` | Allow fonts from same origin, data URLs |
| `worker-src` | `'self' blob:` | Allow web workers |

### Security Analysis

**Q: Does `'wasm-unsafe-eval'` make us vulnerable to XSS?**

**A: No.** Here's why:

1. `'wasm-unsafe-eval'` only allows WebAssembly compilation, not string `eval()`
2. We did NOT add `'unsafe-eval'` (the dangerous one)
3. React escapes all dynamic content by default
4. We don't use `dangerouslySetInnerHTML` anywhere

**What's still blocked (good for security)**:
- ❌ `eval()` on strings
- ❌ Inline scripts injected by attackers
- ❌ Loading scripts from unknown domains
- ❌ `new Function()` with strings

**What's allowed (needed for functionality)**:
- ✅ WebAssembly (Stellar SDK)
- ✅ Scripts from our own domain
- ✅ API calls to our backend
- ✅ Stellar network requests

---

## Testing the Fixes

### 1. CORS Fix Verification

After deploying to Render:

```bash
# Test from browser console on 4everland site
fetch('https://polypulse-backend-436v.onrender.com/api/v1/polls?status=open')
  .then(r => r.json())
  .then(console.log)
```

Should return poll data without CORS errors.

### 2. CSP Fix Verification

After deploying to 4everland:

1. Open browser DevTools Console
2. Look for CSP violation errors
3. Should see no `eval()` blocking errors
4. App should load and function normally

---

## Deployment Checklist

### Backend (Render)

- [x] Update `backend/src/routes/mod.rs` with predicate-based CORS
- [x] Commit and push to main
- [x] Render auto-deploys from main branch
- [x] Verify deployment completes successfully

### Frontend (4everland)

- [x] Update `frontend/vite.config.ts` to exclude `vm` polyfill
- [x] Update `frontend/index.html` with explicit CSP meta tag
- [x] Commit and push to main
- [x] 4everland auto-deploys from main branch
- [x] Verify new deployment URL works

---

## Key Learnings

### 1. CORS is Origin-Specific

CORS checks are **exact string matches** by default. When your deployment URL changes, you need either:
- A wildcard/predicate approach (what we did)
- A custom domain that never changes

### 2. IPFS Gateways Add Security Headers

IPFS gateways like 4everland inject their own CSP headers. You can override them by setting your own CSP meta tag in `index.html`.

### 3. Node.js Polyfills Can Break CSP

When using Node.js libraries in the browser (like Stellar SDK), polyfill plugins may include eval-based code. Always check what you're polyfilling and exclude unnecessary modules.

### 4. `'wasm-unsafe-eval'` ≠ `'unsafe-eval'`

- `'unsafe-eval'`: Dangerous, allows `eval()` on strings
- `'wasm-unsafe-eval'`: Safe, only allows WebAssembly compilation

### 5. React Provides XSS Protection

React's default behavior escapes all dynamic content, so even without CSP, you're protected from most XSS attacks. CSP is an additional layer of defense.

---

## Future Improvements

### Option 1: Custom Domain (Recommended)

Set up a custom domain on 4everland (e.g., `app.polypulse.co.ke`):
- URL never changes
- Can simplify CORS config back to exact list
- More professional appearance

### Option 2: Tighten CSP Further

Once you verify everything works, you can:
- Remove `'unsafe-inline'` from `style-src` by using CSS modules
- Add `nonce` or `hash` based script loading
- Add `report-uri` to monitor CSP violations

### Option 3: Backend CSP Cleanup

The backend's CSP header (`default-src 'none'`) is overly strict for an API. Consider:
- Removing it entirely (APIs don't need CSP)
- Or making it API-specific: `default-src 'none'; frame-ancestors 'none'` is fine for JSON responses

---

## References

- [MDN: CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS)
- [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
- [Tower-HTTP CORS Docs](https://docs.rs/tower-http/latest/tower_http/cors/)
- [Vite Node Polyfills Plugin](https://github.com/davidmyersdev/vite-plugin-node-polyfills)

---

## Commit History

1. `fix: allow all *.ipfs.4everland.app origins via CORS predicate` (641922c)
2. `fix: exclude vm polyfill to avoid eval CSP violation, add explicit CSP meta tag` (40d799b)
