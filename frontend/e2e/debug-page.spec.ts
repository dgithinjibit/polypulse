import { test, expect } from '@playwright/test';

test('debug - capture login page screenshot and DOM', async ({ page }) => {
  await page.goto('/login');
  await page.waitForLoadState('networkidle');
  
  // Take screenshot
  await page.screenshot({ path: 'test-results/login-page-debug.png', fullPage: true });
  
  // Dump all buttons
  const buttons = await page.locator('button').all();
  console.log(`Found ${buttons.length} buttons:`);
  for (const btn of buttons) {
    const text = await btn.textContent();
    const ariaLabel = await btn.getAttribute('aria-label');
    console.log(`  - text: "${text?.trim()}" | aria-label: "${ariaLabel}"`);
  }
  
  // Dump page title and URL
  console.log('URL:', page.url());
  console.log('Title:', await page.title());
  
  // Dump visible text
  const bodyText = await page.locator('body').textContent();
  console.log('Body text (first 500 chars):', bodyText?.slice(0, 500));
  
  // Check for any errors in console
  page.on('console', msg => {
    if (msg.type() === 'error') console.log('Console error:', msg.text());
  });
  
  expect(true).toBe(true);
});

test('debug - capture home page screenshot', async ({ page }) => {
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  
  await page.screenshot({ path: 'test-results/home-page-debug.png', fullPage: true });
  
  const buttons = await page.locator('button').all();
  console.log(`Found ${buttons.length} buttons on home:`);
  for (const btn of buttons) {
    const text = await btn.textContent();
    const ariaLabel = await btn.getAttribute('aria-label');
    console.log(`  - text: "${text?.trim()}" | aria-label: "${ariaLabel}"`);
  }
  
  console.log('URL:', page.url());
  
  expect(true).toBe(true);
});
