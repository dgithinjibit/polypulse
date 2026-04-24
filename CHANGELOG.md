# Changelog

All notable changes to PolyPulse will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2024-04-24

### Added

#### Smart Contracts
- **Multi-Participant Betting Pool Contract** (`contracts/contracts/multi-pool/`)
  - Unlimited participants per pool
  - Separate Yes/No position tracking
  - Proportional payout calculation with 7% platform fee
  - Real-time odds calculation
  - Property-based tested for mathematical correctness
  - State management: Open → Active → Ended → Verified → Paid

#### Backend Services
- **Activity Feed Service** (`backend/src/services/activity_feed.rs`)
  - Record activity events (bet created, participant joined, outcome verified, payout executed)
  - Get recent activities with pagination
  - Get trending bets (most active in last 24h)
  - Get bet activity count
  - Get user activity history

- **Bet Templates Service** (`backend/src/services/bet_templates.rs`)
  - Get templates by category (Crypto, Sports, Weather, General)
  - Fill template with variables
  - Increment usage count
  - Create custom templates
  - Variable validation with property-based tests

- **Reputation System** (`backend/src/services/reputation.rs`)
  - User reputation scores (0-100) with automatic bounds checking
  - Badge system: 🟢 Trusted (80-100), 🟡 Neutral (50-79), 🔴 Caution (<50)
  - Verified user status (90+ reputation, 50+ bets)
  - Minimum reputation requirement (30) for bet creation
  - Reputation event history tracking

- **Leaderboard Service** (`backend/src/services/leaderboard.rs`)
  - Top earners leaderboard
  - Best predictors leaderboard
  - Most active users leaderboard
  - XP system with levels: Bronze (0-99), Silver (100-499), Gold (500-1999), Diamond (2000+)
  - Achievement awards
  - Win streak tracking

- **Telegram Bot Service** (`backend/src/services/telegram_bot.rs`)
  - Bot commands: /start, /bet, /mybets, /positions, /leaderboard, /help
  - Create bets via Telegram
  - Join bets via shareable links
  - Notifications for bet events
  - Wallet linking

#### Database Schema
- **New Tables**:
  - `activities` - Activity feed events
  - `bet_templates` - Bet template library
  - `telegram_users` - Telegram bot integration
  - `achievements` - Achievement definitions
  - `user_achievements` - User achievement unlocks
  - `reputation_events` - Reputation change history
  - `push_subscriptions` - PWA push notifications

- **User Table Extensions**:
  - `xp` - Experience points
  - `level` - User level (1-4)
  - `win_streak` - Consecutive correct predictions
  - `is_premium` - Premium subscription status
  - `premium_activated_at` - Premium activation timestamp
  - `reputation_score` - Reputation score (0-100)
  - `is_verified` - Verified user badge

- **P2P Bets Table Extensions**:
  - `is_multi_participant` - Multi-participant pool flag
  - `total_yes_stakes` - Total stakes on Yes position
  - `total_no_stakes` - Total stakes on No position
  - `participant_count` - Number of participants

- **Default Data**:
  - 5 default achievements
  - 4 default bet templates (Crypto, Sports, Weather, General)

#### Frontend Components
- **Leaderboard Component** (`frontend/src/components/Leaderboard.tsx`)
  - Display top earners, best predictors, most active users
  - Show user levels and XP
  - Medal display for top 3
  - Level badges (Bronze/Silver/Gold/Diamond)

- **PWA Support**:
  - Service worker (`frontend/public/sw.js`) with caching and push notifications
  - PWA manifest (`frontend/public/manifest.json`)
  - PWA service (`frontend/src/services/pwa.ts`) for registration and notifications
  - Offline support for cached bets
  - Install prompt handling

#### API Endpoints
- `GET /activities` - Get recent activities
- `GET /activities/trending` - Get trending bets
- `GET /activities/user/:id` - Get user activity history
- `GET /templates` - Get all bet templates
- `GET /templates/:category` - Get templates by category
- `POST /templates/:id/use` - Increment template usage
- `GET /reputation/:user_id` - Get user reputation
- `GET /reputation/:user_id/history` - Get reputation history
- `GET /leaderboard/:category` - Get leaderboard
- `GET /multi-pools` - List multi-participant pools
- `GET /multi-pools/:id` - Get pool details
- `POST /multi-pools` - Create multi-participant pool
- `POST /multi-pools/:id/join` - Join pool with position and stake
- `GET /multi-pools/:id/odds` - Get current odds

#### Testing
- **Property-Based Tests**:
  - Multi-participant payout fairness (Property 1)
  - Template variable substitution (Property 6)
  - Reputation score bounds (Property 7)
  - Platform fee consistency (7%)
  - Proportional payout calculations
  - Odds calculation accuracy

- **Test Files**:
  - `contracts/contracts/multi-pool/src/property_tests.rs`
  - `backend/tests/multi_pool_payout_tests.rs`
  - Unit tests in all service files

#### Documentation
- `ENHANCEMENT_IMPLEMENTATION_STATUS.md` - Feature implementation tracking
- `PRODUCTION_CHECKLIST.md` - Deployment checklist
- Updated `README.md` with new features and API endpoints
- `CHANGELOG.md` - This file

### Changed
- Updated `README.md` with v2.0 features section
- Updated project structure documentation
- Updated API reference with new endpoints
- Updated smart contracts section with multi-pool contract
- Updated testing section with property-based tests

### Technical Details

#### Property-Based Testing
All mathematical calculations are verified with property-based tests:
- **Property 1**: Multi-Participant Payout Fairness
  - Formula: `(user_stake / total_winning_stakes) * (total_pool * 0.93)`
  - Verified with multiple stake ratios and participant counts
  - All tests passing ✓

- **Property 6**: Template Variable Substitution
  - All placeholders replaced with provided values
  - No remaining placeholder syntax after substitution
  - All tests passing ✓

- **Property 7**: Reputation Score Bounds
  - Score always remains within 0-100 range
  - Regardless of number or magnitude of events
  - All tests passing ✓

#### Performance
- Activity feed queries optimized with indexes
- Leaderboard caching (1 min TTL)
- Template caching (1 hour TTL)
- Real-time updates via WebSocket

#### Security
- Reputation score bounds enforced at database level
- Template variable validation prevents injection
- Multi-pool contract tested for payout accuracy
- All user inputs sanitized

### Migration Guide

#### Database Migration
```bash
cd backend
sqlx migrate run
```

This will create:
- 7 new tables
- 8 new user columns
- 4 new p2p_bets columns
- Default achievements and templates

#### Smart Contract Deployment
```bash
cd contracts
stellar contract build
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm \
  --network mainnet
```

#### Environment Variables
Add to `.env`:
```
STELLAR_MULTI_POOL_CONTRACT_ID=<contract_id>
```

Add to `frontend/.env.production`:
```
VITE_STELLAR_MULTI_POOL_CONTRACT_ID=<contract_id>
```

### Breaking Changes
None. All changes are additive and backward compatible.

### Deprecations
None.

---

## [1.0.0] - 2024-03-01

### Added
- Initial release
- LMSR prediction markets
- 1v1 challenge/wager system
- Stellar wallet integration (Freighter)
- JWT authentication
- WebSocket real-time updates
- PostgreSQL database
- Redis caching
- Rate limiting
- Email verification
- Portfolio tracking
- Transaction history

---

[2.0.0]: https://github.com/polypulse/polypulse/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/polypulse/polypulse/releases/tag/v1.0.0
