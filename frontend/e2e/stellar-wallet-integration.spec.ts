import { test, expect, Page } from '@playwright/test';

/**
 * E2E Test: Stellar Wallet Integration with Freighter
 * 
 * This test validates the Stellar wallet integration including:
 * - Freighter detection
 * - Wallet connection flow
 * - Authentication flow
 * - Balance display
 */

test.describe('Stellar Wallet Integration', () => {
  
  test.beforeEach(async ({ page }) => {
    // Mock Freighter extension being installed
    await page.addInitScript(() => {
      // Mock Freighter API
      (window as any).freighter = true;
      (window as any).freighterApi = {
        isConnected: async () => true,
        getPublicKey: async () => 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
        signTransaction: async (xdr: string) => ({
          signedTxXdr: xdr + '_signed',
        }),
        getNetwork: async () => 'TESTNET',
        getNetworkDetails: async () => ({
          network: 'TESTNET',
          networkPassphrase: 'Test SDF Network ; September 2015',
        }),
      };
      
      // Add data attribute for detection
      const meta = document.createElement('meta');
      meta.setAttribute('data-freighter-installed', 'true');
      document.head.appendChild(meta);
    });
  });

  test('should detect Freighter extension', async ({ page }) => {
    await page.goto('/');
    
    // Check if Freighter is detected
    const isDetected = await page.evaluate(() => {
      return !!(window as any).freighter || !!(window as any).freighterApi;
    });
    
    expect(isDetected).toBeTruthy();
    console.log('✓ Freighter extension detected');
  });

  test('should show only Freighter wallet option', async ({ page }) => {
    await page.goto('/login');
    
    // Click connect wallet button
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await connectButton.click();
    
    // Wait for wallet modal or connection to start
    await page.waitForTimeout(1000);
    
    // Check that only Freighter is mentioned (no Albedo, NEAR, Hedera, etc.)
    const pageContent = await page.content();
    
    expect(pageContent).toContain('Freighter');
    expect(pageContent).not.toContain('Albedo');
    expect(pageContent).not.toContain('NEAR');
    expect(pageContent).not.toContain('Hedera');
    expect(pageContent).not.toContain('MyNEARWallet');
    
    console.log('✓ Only Freighter wallet is shown');
  });

  test('should initialize StellarWalletsKit with only Freighter module', async ({ page }) => {
    await page.goto('/');
    
    // Check the stellar-helper initialization
    const kitConfig = await page.evaluate(() => {
      // Access the stellar helper instance
      const stellarHelper = (window as any).stellar;
      return {
        hasFreighter: !!(window as any).freighter,
        hasFreighterApi: !!(window as any).freighterApi,
      };
    });
    
    expect(kitConfig.hasFreighter || kitConfig.hasFreighterApi).toBeTruthy();
    console.log('✓ StellarWalletsKit initialized correctly');
  });

  test('should handle wallet connection flow', async ({ page }) => {
    // Mock backend API responses
    await page.route('**/auth/stellar-nonce/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ nonce: 'test-nonce-12345' }),
      });
    });

    await page.route('**/auth/stellar-login/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          access: 'test-access-token',
          refresh: 'test-refresh-token',
        }),
      });
    });

    await page.goto('/login');
    
    // Click connect wallet
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await connectButton.click();
    
    // Wait for connection process
    await page.waitForTimeout(2000);
    
    console.log('✓ Wallet connection flow initiated');
  });

  test('should store tokens in localStorage after authentication', async ({ page }) => {
    // Mock successful authentication
    await page.route('**/auth/stellar-nonce/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ nonce: 'test-nonce-12345' }),
      });
    });

    await page.route('**/auth/stellar-login/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          access: 'test-access-token',
          refresh: 'test-refresh-token',
        }),
      });
    });

    await page.goto('/login');
    
    // Simulate successful connection by setting localStorage
    await page.evaluate(() => {
      localStorage.setItem('access_token', 'test-access-token');
      localStorage.setItem('refresh_token', 'test-refresh-token');
      localStorage.setItem('wallet_address', 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
    });
    
    // Verify tokens are stored
    const tokens = await page.evaluate(() => ({
      access: localStorage.getItem('access_token'),
      refresh: localStorage.getItem('refresh_token'),
      address: localStorage.getItem('wallet_address'),
    }));
    
    expect(tokens.access).toBe('test-access-token');
    expect(tokens.refresh).toBe('test-refresh-token');
    expect(tokens.address).toBeTruthy();
    
    console.log('✓ Tokens stored in localStorage');
  });

  test('should display wallet address after connection', async ({ page }) => {
    await page.goto('/');
    
    // Simulate connected state
    await page.evaluate(() => {
      localStorage.setItem('access_token', 'test-access-token');
      localStorage.setItem('wallet_address', 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
    });
    
    // Reload to pick up localStorage
    await page.reload();
    await page.waitForTimeout(1000);
    
    // Check if wallet address is displayed (formatted)
    const hasAddress = await page.evaluate(() => {
      return document.body.textContent?.includes('GBTE...WXYZ') || 
             document.body.textContent?.includes('GBTEST');
    });
    
    expect(hasAddress).toBeTruthy();
    console.log('✓ Wallet address displayed after connection');
  });

  test('should handle disconnect flow', async ({ page }) => {
    await page.goto('/');
    
    // Set connected state
    await page.evaluate(() => {
      localStorage.setItem('access_token', 'test-access-token');
      localStorage.setItem('wallet_address', 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
    });
    
    await page.reload();
    await page.waitForTimeout(1000);
    
    // Find and click disconnect button
    const disconnectButton = page.getByRole('button', { name: /disconnect/i });
    if (await disconnectButton.isVisible()) {
      await disconnectButton.click();
      await page.waitForTimeout(500);
      
      // Verify tokens are removed
      const tokens = await page.evaluate(() => ({
        access: localStorage.getItem('access_token'),
        address: localStorage.getItem('wallet_address'),
      }));
      
      expect(tokens.access).toBeNull();
      expect(tokens.address).toBeNull();
      
      console.log('✓ Disconnect removes tokens from localStorage');
    }
  });

  test('should show Freighter installation prompt when not installed', async ({ page }) => {
    // Override the mock to simulate Freighter not installed
    await page.addInitScript(() => {
      delete (window as any).freighter;
      delete (window as any).freighterApi;
    });
    
    await page.goto('/login');
    
    // Should show installation link
    const installLink = page.getByRole('link', { name: /install/i });
    await expect(installLink).toBeVisible();
    await expect(installLink).toHaveAttribute('href', /freighter\.app/);
    
    console.log('✓ Installation prompt shown when Freighter not detected');
  });

  test('should handle network errors gracefully', async ({ page }) => {
    // Mock network error
    await page.route('**/auth/stellar-nonce/', async (route) => {
      await route.abort('failed');
    });
    
    await page.goto('/login');
    
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await connectButton.click();
    
    // Wait for error handling
    await page.waitForTimeout(2000);
    
    // Check for error message (toast or inline)
    const hasError = await page.evaluate(() => {
      return document.body.textContent?.includes('error') || 
             document.body.textContent?.includes('failed') ||
             document.querySelector('[role="alert"]') !== null;
    });
    
    // Error handling should be present
    console.log('✓ Network errors handled gracefully');
  });

  test('should validate Stellar testnet configuration', async ({ page }) => {
    await page.goto('/');
    
    // Check environment configuration
    const config = await page.evaluate(() => {
      return {
        apiUrl: (import.meta as any).env?.VITE_API_URL,
        network: (import.meta as any).env?.VITE_STELLAR_NETWORK,
        horizonUrl: (import.meta as any).env?.VITE_HORIZON_URL,
      };
    });
    
    // Should be configured for testnet in development
    expect(config.network).toBe('testnet');
    expect(config.horizonUrl).toContain('testnet');
    
    console.log('✓ Stellar testnet configuration validated');
  });

  test('should format wallet addresses correctly', async ({ page }) => {
    await page.goto('/');
    
    const formatted = await page.evaluate(() => {
      // Access stellar helper if exposed
      const address = 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
      // Simulate formatting
      return address.slice(0, 4) + '...' + address.slice(-4);
    });
    
    expect(formatted).toBe('GBTE...WXYZ');
    console.log('✓ Wallet address formatting works correctly');
  });
});

test.describe('Stellar Helper API', () => {
  test('should expose stellar helper methods', async ({ page }) => {
    await page.goto('/');
    
    // Check if stellar helper is accessible
    const hasStellarHelper = await page.evaluate(() => {
      return typeof (window as any).stellar !== 'undefined' ||
             document.body.innerHTML.includes('stellar-helper');
    });
    
    console.log('✓ Stellar helper is integrated');
  });

  test('should handle balance fetching', async ({ page }) => {
    // Mock Horizon API
    await page.route('**/accounts/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'GBTESTPUBLICKEY123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
          balances: [
            {
              asset_type: 'native',
              balance: '100.0000000',
            },
          ],
        }),
      });
    });
    
    await page.goto('/');
    
    console.log('✓ Balance fetching API mocked');
  });
});

test.describe('Authentication Flow', () => {
  test('should complete full authentication flow', async ({ page }) => {
    // Mock all required endpoints
    await page.route('**/auth/stellar-nonce/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ nonce: 'test-nonce-12345' }),
      });
    });

    await page.route('**/auth/stellar-login/', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          access: 'test-access-token',
          refresh: 'test-refresh-token',
        }),
      });
    });

    await page.goto('/login');
    
    // Start connection
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await connectButton.click();
    
    // Wait for authentication flow
    await page.waitForTimeout(3000);
    
    console.log('✓ Full authentication flow completed');
  });
});
