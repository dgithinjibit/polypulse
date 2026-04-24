# Requirements Document: Backend Rust Compilation Fix

## Introduction

The PolyPulse backend (Rust/Axum) is currently failing to compile on Render due to 32 compilation errors in the Telegram bot service. This prevents the backend from deploying, which blocks all authentication and API functionality. This spec defines the requirements to fix all compilation errors and successfully deploy the backend to Render.

## Problem Statement

**Current Status**: Backend build fails on Render with exit code 101

**Error Summary**:
- File: `backend/src/services/telegram_bot.rs`
- Total Errors: 32 compilation errors
- Primary Issue: "value used here after partial move" (E0382)
- Secondary Issues: Unused variables, type mismatches

**Impact**:
- Backend cannot deploy to Render
- Frontend cannot authenticate users
- No API endpoints available
- Platform is non-functional

**Root Cause**: 
On line 89, `msg.text` is moved with `.ok_or()`, but then on line 143 and other locations, it's used again with `.unwrap()` or accessed directly. Rust's ownership system prevents using a value after it has been moved.

## Glossary

- **Rust Ownership**: Rust's memory safety system where each value has a single owner
- **Move Semantics**: When a value is moved, the original variable can no longer be used
- **Borrow**: Temporarily accessing a value without taking ownership
- **Clone**: Creating a deep copy of a value to avoid move issues
- **Render**: Cloud platform hosting the PolyPulse backend
- **Telegram Bot**: Service that handles Telegram webhook messages and commands
- **Compilation Error E0382**: "Use of moved value" error in Rust

## Requirements

### Requirement 1: Fix Message Text Ownership Issues

**User Story**: As a developer, I want the Telegram bot to compile successfully, so that the backend can deploy to Render.

#### Acceptance Criteria

1. THE System SHALL fix all "value used after move" errors in `telegram_bot.rs`
2. WHEN `msg.text` is accessed, THE System SHALL use `.clone()` or borrowing to avoid move
3. THE System SHALL ensure `msg.text` is available for all command handlers
4. THE System SHALL maintain the same functionality after fixes
5. ALL compilation errors related to `msg.text` SHALL be resolved
6. THE code SHALL follow Rust best practices for ownership

**Example Fix**:
```rust
// Before (causes error)
let text = msg.text.ok_or(BotError::NoText)?;
// ... later ...
let text2 = msg.text.unwrap(); // ERROR: msg.text was moved

// After (correct)
let text = msg.text.clone().ok_or(BotError::NoText)?;
// ... later ...
let text2 = msg.text.unwrap(); // OK: msg.text was cloned, not moved
```

### Requirement 2: Fix Unused Variable Warnings

**User Story**: As a developer, I want all unused variable warnings resolved, so that the code is clean and maintainable.

#### Acceptance Criteria

1. THE System SHALL fix all unused variable warnings in `telegram_bot.rs`
2. WHEN a variable is unused, THE System SHALL either use it or prefix with underscore
3. THE System SHALL remove truly unnecessary variables
4. THE code SHALL compile with zero warnings
5. THE System SHALL maintain code readability

**Example Fix**:
```rust
// Before (warning)
let share_url = format!("https://t.me/polypulse_bot?start=bet_{}", bet_id);
// share_url is never used

// After (fixed)
let _share_url = format!("https://t.me/polypulse_bot?start=bet_{}", bet_id);
// OR remove if truly unnecessary
```

### Requirement 3: Verify Compilation Success

**User Story**: As a developer, I want to verify the backend compiles successfully, so that I can confidently deploy to Render.

#### Acceptance Criteria

1. THE System SHALL run `cargo build --release` successfully
2. THE build SHALL complete with exit code 0
3. THE build SHALL produce zero errors
4. THE build SHALL produce zero warnings (or only acceptable warnings)
5. THE System SHALL verify all tests still pass after fixes
6. THE System SHALL document any behavioral changes

**Test Command**:
```bash
cd backend
cargo build --release
# Expected: exit code 0, no errors
```

### Requirement 4: Deploy to Render

**User Story**: As a platform operator, I want the backend deployed to Render, so that the frontend can authenticate users and access APIs.

#### Acceptance Criteria

1. WHEN code is pushed to GitHub, Render SHALL automatically trigger a build
2. THE Render build SHALL complete successfully
3. THE backend service SHALL start and respond to health checks
4. THE System SHALL verify API endpoints are accessible
5. THE System SHALL verify database migrations run successfully
6. THE System SHALL verify environment variables are configured correctly

**Verification**:
```bash
# Test health endpoint
curl https://polypulse-backend-436v.onrender.com/health
# Expected: 200 OK

# Test API endpoint
curl https://polypulse-backend-436v.onrender.com/api/v1/auth/stellar-nonce \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"public_key": "GTEST..."}'
# Expected: 200 OK with nonce
```

### Requirement 5: Maintain Telegram Bot Functionality

**User Story**: As a Telegram user, I want the bot commands to work correctly after fixes, so that I can create and manage bets via Telegram.

#### Acceptance Criteria

1. THE `/start` command SHALL work correctly
2. THE `/bet` command SHALL parse and create bets
3. THE `/mybets` command SHALL display user's bets
4. THE `/positions` command SHALL display user's positions
5. THE `/leaderboard` command SHALL display top users
6. THE `/help` command SHALL display help message
7. ALL command parsing SHALL handle edge cases (missing args, invalid format)
8. THE System SHALL maintain error handling for invalid commands

### Requirement 6: Document Changes

**User Story**: As a developer, I want clear documentation of all changes, so that I understand what was fixed and why.

#### Acceptance Criteria

1. THE System SHALL create a fix summary document
2. THE document SHALL list all errors fixed
3. THE document SHALL explain the root cause of each error
4. THE document SHALL show before/after code examples
5. THE document SHALL document any behavioral changes
6. THE document SHALL include verification steps
7. THE document SHALL be committed with the fixes

## Error Details

### Primary Errors (from Render build log)

**Error 1: Line 89 - msg.text moved**
```
error[E0382]: use of partially moved value: `msg`
  --> src/services/telegram_bot.rs:89:9
   |
89 |         let text = msg.text.ok_or(BotError::NoText)?;
   |                    -------- value moved here
...
143|         let text = msg.text.unwrap();
   |                    ^^^^^^^^ value used here after partial move
```

**Error 2: Line 348 - unused variable**
```
warning: unused variable: `share_url`
  --> src/services/telegram_bot.rs:348:21
   |
348|                 let share_url = format!("https://t.me/polypulse_bot?start=bet_{}", bet_id);
   |                     ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_share_url`
```

### Expected Fixes

1. **Clone msg.text before moving**: Use `.clone()` to create a copy
2. **Use references where possible**: Borrow instead of move when appropriate
3. **Prefix unused variables**: Add `_` prefix or remove if unnecessary
4. **Restructure code**: Refactor to avoid multiple accesses to moved values

## Success Criteria

### Build Success
- ✅ `cargo build --release` exits with code 0
- ✅ Zero compilation errors
- ✅ Zero warnings (or only acceptable warnings)
- ✅ Binary produced in `target/release/backend`

### Deployment Success
- ✅ Render build completes successfully
- ✅ Backend service starts and stays running
- ✅ Health endpoint returns 200 OK
- ✅ API endpoints respond correctly
- ✅ Database connection works

### Functionality Preserved
- ✅ All Telegram bot commands work
- ✅ Message parsing works correctly
- ✅ Error handling works correctly
- ✅ No behavioral changes (unless documented)

## Out of Scope

- Adding new Telegram bot features
- Refactoring unrelated code
- Performance optimizations
- Adding new tests (unless needed to verify fixes)
- Changing API endpoints
- Modifying database schema

## Dependencies

- Rust toolchain (already installed on Render)
- PostgreSQL database (already configured)
- Render deployment environment (already set up)
- GitHub repository (already connected to Render)

## Risks

1. **Risk**: Fixes might change bot behavior
   - **Mitigation**: Test all commands after fixes
   
2. **Risk**: Other compilation errors might appear
   - **Mitigation**: Fix iteratively, test after each fix

3. **Risk**: Render deployment might fail for other reasons
   - **Mitigation**: Check Render logs, verify environment variables

## Timeline

- **Estimated Time**: 30-60 minutes
- **Phase 1**: Fix compilation errors (20 minutes)
- **Phase 2**: Test locally (10 minutes)
- **Phase 3**: Deploy to Render (10 minutes)
- **Phase 4**: Verify deployment (10 minutes)

## Verification Steps

1. **Local Verification**:
   ```bash
   cd backend
   cargo clean
   cargo build --release
   cargo test
   ```

2. **Render Verification**:
   - Push to GitHub
   - Monitor Render build logs
   - Check service status
   - Test API endpoints

3. **Functional Verification**:
   - Test frontend authentication
   - Test API endpoints via curl
   - Test Telegram bot commands (if webhook configured)

## Related Documents

- `CURRENT_FEATURES_SUMMARY.md` - Current platform status
- `backend/src/services/telegram_bot.rs` - File to fix
- Render build logs - Error details
