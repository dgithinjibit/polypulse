import { test, expect } from '@playwright/test';
import { ErrorCapture } from './helpers/error-capture';

test.describe('Example with Error Capture', () => {
  test('should capture and report errors', async ({ page }) => {
    // Initialize error capture
    const errorCapture = new ErrorCapture(page);

    // Navigate to your app
    await page.goto('/');

    // Perform your test actions
    // ... your test code here ...

    // Check for errors at the end
    if (errorCapture.hasErrors()) {
      errorCapture.printErrors();
      
      // Optionally fail the test if there are errors
      // expect(errorCapture.getErrors()).toHaveLength(0);
    }
  });

  test('should navigate and check for console errors', async ({ page }) => {
    const errorCapture = new ErrorCapture(page);

    await page.goto('/');
    
    // Wait for page to be fully loaded
    await page.waitForLoadState('networkidle');

    // Get all errors
    const errors = errorCapture.getErrors();
    
    // Log errors for debugging
    if (errors.length > 0) {
      console.log('Found errors:', errors);
    }

    // Assert no critical errors
    const criticalErrors = errors.filter(e => 
      e.message.includes('TypeError') || 
      e.message.includes('ReferenceError')
    );
    
    expect(criticalErrors).toHaveLength(0);
  });
});
