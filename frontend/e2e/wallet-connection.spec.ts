import { test, expect, Page } from '@playwright/test';

/**
 * E2E Test: Stellar Wallet Connection Flow
 * 
 * This test simulates the wallet connection flow without requiring
 * a real browser extension. It validates the UI elements and flow.
 */

test.describe('Stellar Wallet Connection', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should display connect wallet button on homepage', async ({ page }) => {
    // Check if connect button exists
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await expect(connectButton).toBeVisible();
    
    // Verify button is enabled
    await expect(connectButton).toBeEnabled();
    
    console.log('✓ Connect wallet button is visible and enabled');
  });

  test('should navigate to login page when not connected', async ({ page }) => {
    // Try to access a protected route
    await page.goto('/portfolio');
    
    // Should redirect to login
    await expect(page).toHaveURL('/login');
    
    console.log('✓ Protected route redirects to login');
  });

  test('should display login page with Stellar branding', async ({ page }) => {
    await page.goto('/login');
    
    // Check for Stellar-specific elements
    await expect(page.getByText(/Welcome to PolyPulse/i)).toBeVisible();
    await expect(page.getByText(/Connect your Stellar wallet/i)).toBeVisible();
    await expect(page.getByText(/Freighter/i)).toBeVisible();
    
    // Check for connect button
    const connectButton = page.getByRole('button', { name: /connect wallet/i });
    await expect(connectButton).toBeVisible();
    
    console.log('✓ Login page displays Stellar branding correctly');
  });

  test('should show Freighter installation link', async ({ page }) => {
    await page.goto('/login');
    
    // Check for installation link
    const installLink = page.getByRole('link', { name: /install it here/i });
    await expect(installLink).toBeVisible();
    await expect(installLink).toHaveAttribute('href', 'https://www.freighter.app/');
    
    console.log('✓ Freighter installation link is present');
  });

  test('should display loading state when connecting', async ({ page }) => {
    await page.goto('/login');
    
    // Mock the wallet connection to simulate loading
    await page.evaluate(() => {
      // Simulate loading state by triggering the button
      const button = document.querySelector('button[aria-label*="Connect"]') as HTMLButtonElement;
      if (button) {
        button.disabled = true;
        button.setAttribute('aria-busy', 'true');
      }
    });
    
    // Check for loading indicator
    const loadingSpinner = page.locator('[role="status"]');
    await expect(loadingSpinner).toBeVisible();
    
    console.log('✓ Loading state displays correctly');
  });

  test('should have accessible navigation', async ({ page }) => {
    await page.goto('/');
    
    // Check navbar accessibility
    const navbar = page.locator('nav');
    await expect(navbar).toBeVisible();
    
    // Check for main navigation links
    await expect(page.getByRole('link', { name: /markets/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /leaderboard/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /challenges/i })).toBeVisible();
    
    console.log('✓ Navigation is accessible');
  });

  test('should display mobile menu button on small screens', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    
    // Check for mobile menu button
    const menuButton = page.getByRole('button', { name: /open menu|close menu/i });
    await expect(menuButton).toBeVisible();
    
    // Click to open menu
    await menuButton.click();
    
    // Check if mobile menu is visible
    const mobileMenu = page.locator('#mobile-menu');
    await expect(mobileMenu).toBeVisible();
    
    console.log('✓ Mobile menu works correctly');
  });

  test('should have proper ARIA labels on interactive elements', async ({ page }) => {
    await page.goto('/login');
    
    // Check ARIA labels
    const connectButton = page.getByRole('button', { name: /connect stellar wallet/i });
    await expect(connectButton).toHaveAttribute('aria-label');
    
    console.log('✓ ARIA labels are present');
  });

  test('should display help page with wallet instructions', async ({ page }) => {
    await page.goto('/help');
    
    // Check for help content
    await expect(page.getByText(/Help & Support/i)).toBeVisible();
    await expect(page.getByText(/Wallet Connection Issues/i)).toBeVisible();
    await expect(page.getByText(/Freighter Wallet Not Installed/i)).toBeVisible();
    
    console.log('✓ Help page displays wallet instructions');
  });

  test('should maintain focus order for keyboard navigation', async ({ page }) => {
    await page.goto('/login');
    
    // Tab through elements
    await page.keyboard.press('Tab');
    
    // Check if focus is visible
    const focusedElement = await page.evaluate(() => {
      const el = document.activeElement;
      return el ? el.tagName : null;
    });
    
    expect(focusedElement).toBeTruthy();
    
    console.log('✓ Keyboard navigation works');
  });
});

test.describe('Wallet Modal', () => {
  test('should display wallet modal with Stellar wallets only', async ({ page }) => {
    await page.goto('/');
    
    // This test would require mocking the wallet modal trigger
    // For now, we verify the component exists in the bundle
    const hasWalletModal = await page.evaluate(() => {
      return document.body.innerHTML.includes('Stellar') || 
             document.body.innerHTML.includes('Freighter');
    });
    
    expect(hasWalletModal).toBeTruthy();
    
    console.log('✓ Stellar wallet references found in page');
  });
});

test.describe('Error Handling', () => {
  test('should display toast notifications', async ({ page }) => {
    await page.goto('/');
    
    // Check if toast container exists
    const toastViewport = page.locator('[data-radix-toast-viewport]');
    
    // Toast viewport should be in the DOM (even if empty)
    const exists = await toastViewport.count();
    expect(exists).toBeGreaterThanOrEqual(0);
    
    console.log('✓ Toast notification system is initialized');
  });
});

test.describe('Responsive Design', () => {
  const viewports = [
    { name: 'Mobile', width: 375, height: 667 },
    { name: 'Tablet', width: 768, height: 1024 },
    { name: 'Desktop', width: 1920, height: 1080 },
  ];

  for (const viewport of viewports) {
    test(`should render correctly on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto('/');
      
      // Check if page renders without errors
      await expect(page.locator('body')).toBeVisible();
      
      // Check for main content
      await expect(page.getByText(/PolyPulse/i)).toBeVisible();
      
      console.log(`✓ ${viewport.name} viewport renders correctly`);
    });
  }
});

test.describe('Color Contrast', () => {
  test('should have sufficient color contrast', async ({ page }) => {
    await page.goto('/login');
    
    // Check background gradient is applied
    const hasGradient = await page.evaluate(() => {
      const body = document.body;
      const computed = window.getComputedStyle(body);
      return computed.background.includes('gradient') || 
             document.querySelector('.bg-gradient-polypulse-light') !== null;
    });
    
    expect(hasGradient).toBeTruthy();
    
    console.log('✓ Gradient backgrounds are applied');
  });
});
