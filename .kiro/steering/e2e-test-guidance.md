---
inclusion: auto
fileMatchPattern: "e2e/**/*.spec.ts"
---

# E2E Test Guidance for Stellar Wallet Connection

## Test Failure Analysis Priority

When E2E tests fail, analyze errors in this order:

### 1. Element Not Found Errors
**Priority**: HIGH
**Common Causes**:
- ARIA label mismatch (check `aria-label` attributes)
- Component not rendered (check conditional rendering)
- Timing issues (add `await expect().toBeVisible()`)

**Fix Strategy**:
1. Check the component file for the correct ARIA label
2. Verify the element exists in the DOM
3. Add proper wait conditions if needed

### 2. Navigation/Routing Errors
**Priority**: HIGH
**Common Causes**:
- Protected route logic incorrect
- Missing redirect in authentication flow
- Route path mismatch

**Fix Strategy**:
1. Check `ProtectedRoute` component logic
2. Verify `StellarWalletContext` `isConnected` state
3. Check route definitions in `App.tsx`

### 3. Wallet Connection Errors
**Priority**: CRITICAL
**Common Causes**:
- Freighter API not mocked properly
- `stellar-helper.ts` connection logic broken
- Context state not updating

**Fix Strategy**:
1. Check `StellarWalletContext.tsx` `connectWallet()` method
2. Verify `stellar-helper.ts` `connectWallet()` implementation
3. Check localStorage persistence logic
4. Verify error handling in context

### 4. Styling/Visual Errors
**Priority**: MEDIUM
**Common Causes**:
- Gradient classes not applied
- Tailwind config issue
- CSS not loaded

**Fix Strategy**:
1. Check `main.css` for gradient utilities
2. Verify Tailwind config includes custom classes
3. Check component className attributes

### 5. Accessibility Errors
**Priority**: HIGH
**Common Causes**:
- Missing ARIA labels
- Incorrect role attributes
- Focus management broken

**Fix Strategy**:
1. Add/fix `aria-label` attributes
2. Add `role` attributes where needed
3. Check focus states with `focus:ring-2`

## Self-Correction Workflow

When tests fail:

1. **Read the error message carefully**
   - Extract the failing test name
   - Identify the assertion that failed
   - Note the expected vs actual values

2. **Locate the relevant component**
   - Map test file to component file
   - Example: `wallet-connection.spec.ts` → `Login.tsx`, `StellarWalletContext.tsx`

3. **Apply the fix**
   - Make minimal changes
   - Follow existing patterns
   - Maintain accessibility standards

4. **Verify the fix**
   - Re-run the specific test
   - Check for regressions
   - Ensure no new errors introduced

## Component-to-Test Mapping

| Test File | Primary Components | Secondary Components |
|-----------|-------------------|---------------------|
| `wallet-connection.spec.ts` | `Login.tsx`, `StellarWalletContext.tsx` | `Navbar.tsx`, `ProtectedRoute.tsx` |
| `wallet-modal.spec.ts` | `WalletModal.tsx` | `stellar-helper.ts` |
| `navigation.spec.ts` | `Navbar.tsx`, `App.tsx` | All page components |

## Common Test Patterns

### Testing Button Visibility
```typescript
const button = page.getByRole('button', { name: /connect wallet/i });
await expect(button).toBeVisible();
```

### Testing Navigation
```typescript
await page.goto('/protected-route');
await expect(page).toHaveURL('/login');
```

### Testing ARIA Labels
```typescript
const element = page.getByRole('button', { name: /specific label/i });
await expect(element).toHaveAttribute('aria-label');
```

## Error Message Patterns

### "Element not found"
→ Check component rendering and ARIA labels

### "Expected URL to be X but got Y"
→ Check routing logic and redirects

### "Timeout waiting for element"
→ Add proper wait conditions or check if element exists

### "Expected element to be visible"
→ Check CSS display properties and conditional rendering

## Stellar-Specific Considerations

1. **Freighter Extension**: Tests run without real extension
   - Mock wallet responses in tests
   - Focus on UI/UX validation
   - Test error states explicitly

2. **Authentication Flow**: Multi-step process
   - Connect wallet → Sign message → Store tokens → Navigate
   - Each step should be testable independently
   - Check loading states between steps

3. **State Persistence**: localStorage usage
   - Tests should clear localStorage between runs
   - Verify state restoration logic
   - Test invalid state handling

## Best Practices

1. **Keep tests focused**: One assertion per test when possible
2. **Use descriptive test names**: Should read like documentation
3. **Add console.log for debugging**: Help identify which tests pass
4. **Test accessibility**: Every interactive element needs ARIA labels
5. **Test responsive design**: Multiple viewport sizes

## When to Skip Auto-Fix

Do NOT auto-fix if:
- Test is checking for intentional error states
- Multiple tests fail in different components (investigate root cause first)
- Error is in test code itself (not component code)
- Breaking change is required (consult user first)
