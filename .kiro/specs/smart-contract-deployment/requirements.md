# Requirements Document: Stellar Smart Contract Deployment

## Introduction

PolyPulse has two Soroban smart contracts written and tested but not yet deployed to the Stellar testnet. These contracts are essential for the platform's betting functionality. This spec defines the requirements to deploy both contracts to Stellar testnet, configure the frontend with contract IDs, and verify end-to-end functionality.

## Problem Statement

**Current Status**: Smart contracts exist in code but are not deployed

**Contracts to Deploy**:
1. **P2P Bet Contract** (`contracts/contracts/market/src/lib.rs`)
   - Purpose: 1-on-1 peer-to-peer betting
   - Status: Written, tested, not deployed
   
2. **Multi-Participant Pool Contract** (`contracts/contracts/multi-pool/src/lib.rs`)
   - Purpose: Unlimited participants per bet
   - Status: Written, tested, not deployed

**Impact**:
- Users cannot create real bets
- Users cannot join bets
- Users cannot report outcomes
- Users cannot receive payouts
- Platform is non-functional for core betting

## Glossary

- **Soroban**: Stellar's smart contract platform
- **Stellar Testnet**: Test network for Stellar blockchain (not real money)
- **Contract ID**: Unique identifier for deployed smart contract
- **WASM**: WebAssembly binary format for smart contracts
- **Freighter**: Browser wallet for Stellar
- **Horizon**: Stellar API server
- **Stellar CLI**: Command-line tool for Stellar operations
- **XLM**: Stellar Lumens (native cryptocurrency)
- **Stroops**: Smallest unit of XLM (1 XLM = 10,000,000 stroops)

## Requirements

### Requirement 1: Set Up Deployment Environment

**User Story**: As a developer, I want a properly configured deployment environment, so that I can deploy contracts to Stellar testnet.

#### Acceptance Criteria

1. THE System SHALL have Stellar CLI installed (version 21.0.0+)
2. THE System SHALL have Rust toolchain installed (version 1.74.0+)
3. THE System SHALL have a funded testnet account for deployment
4. THE testnet account SHALL have at least 10,000 XLM for deployment fees
5. THE System SHALL configure Stellar CLI for testnet network
6. THE System SHALL verify network connectivity to Stellar testnet

**Installation Commands**:
```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Verify installation
stellar --version
# Expected: stellar 21.x.x

# Configure for testnet
stellar network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Create deployment identity
stellar keys generate deployer --network testnet

# Fund account from friendbot
stellar keys fund deployer --network testnet
```

### Requirement 2: Build Smart Contracts

**User Story**: As a developer, I want to build optimized WASM binaries, so that contracts can be deployed efficiently.

#### Acceptance Criteria

1. THE System SHALL build P2P bet contract to WASM
2. THE System SHALL build multi-pool contract to WASM
3. THE builds SHALL use release mode for optimization
4. THE builds SHALL produce `.wasm` files in `target/wasm32-unknown-unknown/release/`
5. THE System SHALL verify WASM files are valid
6. THE System SHALL optimize WASM files for size

**Build Commands**:
```bash
# Build P2P bet contract
cd contracts/contracts/market
stellar contract build

# Build multi-pool contract
cd ../multi-pool
stellar contract build

# Verify builds
ls -lh ../../target/wasm32-unknown-unknown/release/*.wasm
```

### Requirement 3: Deploy P2P Bet Contract

**User Story**: As a platform operator, I want the P2P bet contract deployed to testnet, so that users can create 1-on-1 bets.

#### Acceptance Criteria

1. THE System SHALL deploy P2P bet contract to Stellar testnet
2. THE deployment SHALL return a unique contract ID
3. THE contract ID SHALL be a valid Stellar contract address (starts with 'C')
4. THE System SHALL verify contract is deployed and accessible
5. THE System SHALL initialize contract if required
6. THE System SHALL save contract ID for frontend configuration
7. THE deployment SHALL complete within 60 seconds

**Deployment Commands**:
```bash
# Deploy contract
cd contracts/contracts/market
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market.wasm \
  --source deployer \
  --network testnet

# Save contract ID
export P2P_CONTRACT_ID="<returned_contract_id>"
echo "P2P Contract ID: $P2P_CONTRACT_ID"

# Verify deployment
stellar contract invoke \
  --id $P2P_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  --help
```

### Requirement 4: Deploy Multi-Pool Contract

**User Story**: As a platform operator, I want the multi-pool contract deployed to testnet, so that users can create multi-participant bets.

#### Acceptance Criteria

1. THE System SHALL deploy multi-pool contract to Stellar testnet
2. THE deployment SHALL return a unique contract ID
3. THE contract ID SHALL be a valid Stellar contract address (starts with 'C')
4. THE System SHALL verify contract is deployed and accessible
5. THE System SHALL initialize contract if required
6. THE System SHALL save contract ID for frontend configuration
7. THE deployment SHALL complete within 60 seconds

**Deployment Commands**:
```bash
# Deploy contract
cd contracts/contracts/multi-pool
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/multi_pool.wasm \
  --source deployer \
  --network testnet

# Save contract ID
export MULTI_POOL_CONTRACT_ID="<returned_contract_id>"
echo "Multi-Pool Contract ID: $MULTI_POOL_CONTRACT_ID"

# Verify deployment
stellar contract invoke \
  --id $MULTI_POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  --help
```

### Requirement 5: Configure Frontend Environment Variables

**User Story**: As a developer, I want the frontend configured with contract IDs, so that it can interact with deployed contracts.

#### Acceptance Criteria

1. THE System SHALL update `.env.production` with P2P contract ID
2. THE System SHALL update `.env.production` with multi-pool contract ID
3. THE System SHALL update 4everland environment variables
4. THE System SHALL verify environment variables are set correctly
5. THE frontend SHALL rebuild with new contract IDs
6. THE System SHALL document contract IDs in README

**Configuration Steps**:
```bash
# Update .env.production
cat >> frontend/.env.production << EOF
VITE_STELLAR_MARKET_CONTRACT_ID=$P2P_CONTRACT_ID
VITE_STELLAR_MULTIPOOL_CONTRACT_ID=$MULTI_POOL_CONTRACT_ID
EOF

# Update 4everland (manual step)
# Go to 4everland dashboard → Environment Variables
# Add:
#   VITE_STELLAR_MARKET_CONTRACT_ID = <P2P_CONTRACT_ID>
#   VITE_STELLAR_MULTIPOOL_CONTRACT_ID = <MULTI_POOL_CONTRACT_ID>

# Trigger redeploy on 4everland
git add frontend/.env.production
git commit -m "feat: Add deployed contract IDs"
git push origin main
```

### Requirement 6: Test Contract Functionality

**User Story**: As a developer, I want to verify contracts work correctly, so that users can bet without issues.

#### Acceptance Criteria

1. THE System SHALL test creating a bet via P2P contract
2. THE System SHALL test joining a bet via P2P contract
3. THE System SHALL test reporting outcome via P2P contract
4. THE System SHALL test payout execution via P2P contract
5. THE System SHALL test creating a multi-pool bet
6. THE System SHALL test joining a multi-pool bet
7. ALL tests SHALL complete successfully
8. THE System SHALL document test results

**Test Commands**:
```bash
# Test P2P contract - Create bet
stellar contract invoke \
  --id $P2P_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  create_bet \
  --creator deployer \
  --question "Will it rain tomorrow?" \
  --stake 1000000 \
  --end_time 1735689600

# Test P2P contract - Join bet
stellar contract invoke \
  --id $P2P_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  join_bet \
  --participant deployer \
  --bet_id 1 \
  --position true

# Test Multi-pool contract - Create pool
stellar contract invoke \
  --id $MULTI_POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  create_pool \
  --creator deployer \
  --question "Will BTC reach $100k?" \
  --end_time 1735689600

# Test Multi-pool contract - Join pool
stellar contract invoke \
  --id $MULTI_POOL_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  join_pool \
  --participant deployer \
  --pool_id 1 \
  --position true \
  --stake 1000000
```

### Requirement 7: End-to-End Frontend Testing

**User Story**: As a user, I want to create and join bets via the frontend, so that I can use the platform naturally.

#### Acceptance Criteria

1. THE frontend SHALL connect to Freighter wallet
2. THE frontend SHALL display deployed contract IDs in console
3. THE user SHALL be able to create a bet via UI
4. THE bet creation SHALL call the deployed P2P contract
5. THE user SHALL be able to join a bet via UI
6. THE bet joining SHALL call the deployed P2P contract
7. THE user SHALL see transaction confirmations
8. THE user SHALL see updated bet status after transactions
9. ALL transactions SHALL complete successfully on testnet

**Testing Steps**:
1. Open frontend on 4everland
2. Connect Freighter wallet (testnet mode)
3. Fund wallet from friendbot if needed
4. Click "Create Bet"
5. Fill in question, stake, end time
6. Submit and sign transaction
7. Verify bet appears in list
8. Join bet with another account
9. Verify participant count updates
10. Report outcome after end time
11. Verify payout execution

### Requirement 8: Document Deployment

**User Story**: As a developer, I want comprehensive deployment documentation, so that I can redeploy or troubleshoot issues.

#### Acceptance Criteria

1. THE System SHALL create a deployment guide document
2. THE document SHALL include all contract IDs
3. THE document SHALL include deployment commands
4. THE document SHALL include verification steps
5. THE document SHALL include troubleshooting tips
6. THE document SHALL include network configuration
7. THE document SHALL be committed to repository

**Document Contents**:
- Contract IDs (P2P and Multi-pool)
- Deployment date and network
- Deployer account address
- Build commands
- Deployment commands
- Test commands
- Frontend configuration steps
- Troubleshooting guide

## Contract Details

### P2P Bet Contract

**Location**: `contracts/contracts/market/src/lib.rs`

**Functions**:
- `create_bet(creator, question, stake, end_time)` - Create 1v1 bet
- `join_bet(participant, bet_id, position)` - Join existing bet
- `cancel_bet(bet_id)` - Cancel bet before participant joins
- `report_outcome(reporter, bet_id, outcome)` - Report outcome
- `confirm_outcome(verifier, bet_id, outcome)` - Verify outcome
- `execute_payout(bet_id)` - Execute payout to winner
- `get_bet(bet_id)` - Get bet details

**Storage**:
- Bet state (Created, Active, Ended, Verified, Paid)
- Participants (creator, joiner)
- Stakes (creator_stake, joiner_stake)
- Outcome reports
- Platform fee (2%)

### Multi-Pool Contract

**Location**: `contracts/contracts/multi-pool/src/lib.rs`

**Functions**:
- `create_pool(creator, question, end_time)` - Create multi-participant pool
- `join_pool(participant, pool_id, position, stake)` - Join pool
- `get_odds(pool_id)` - Get current odds
- `report_outcome(reporter, pool_id, outcome)` - Report outcome
- `distribute_payouts(pool_id)` - Execute proportional payouts

**Storage**:
- Pool state (Open, Active, Ended, Verified, Paid)
- Yes participants (addresses + stakes)
- No participants (addresses + stakes)
- Total stakes (yes_total, no_total)
- Platform fee (7%)

## Success Criteria

### Deployment Success
- ✅ Both contracts deployed to testnet
- ✅ Contract IDs obtained and saved
- ✅ Contracts accessible via Stellar CLI
- ✅ Contracts respond to function calls

### Configuration Success
- ✅ Frontend environment variables updated
- ✅ 4everland environment variables updated
- ✅ Frontend rebuilt with new contract IDs
- ✅ Contract IDs visible in browser console

### Functionality Success
- ✅ Can create bet via CLI
- ✅ Can join bet via CLI
- ✅ Can create bet via frontend
- ✅ Can join bet via frontend
- ✅ Transactions confirm on testnet
- ✅ Bet state updates correctly

### Documentation Success
- ✅ Deployment guide created
- ✅ Contract IDs documented
- ✅ Commands documented
- ✅ Troubleshooting guide included

## Out of Scope

- Deploying to mainnet (testnet only)
- Contract upgrades or migrations
- Adding new contract functions
- Performance optimization
- Gas cost optimization
- Contract auditing
- Mainnet deployment planning

## Dependencies

- Stellar CLI (version 21.0.0+)
- Rust toolchain (version 1.74.0+)
- Funded testnet account (10,000+ XLM)
- Freighter wallet (for testing)
- 4everland deployment (for frontend)
- GitHub repository (for version control)

## Risks

1. **Risk**: Deployment might fail due to network issues
   - **Mitigation**: Retry deployment, check network status

2. **Risk**: Contract might have bugs discovered after deployment
   - **Mitigation**: Test thoroughly before deployment, use testnet first

3. **Risk**: Frontend might not connect to contracts
   - **Mitigation**: Verify contract IDs, check network configuration

4. **Risk**: Transactions might fail due to insufficient balance
   - **Mitigation**: Fund accounts from friendbot, check balances

## Timeline

- **Estimated Time**: 60-90 minutes
- **Phase 1**: Environment setup (15 minutes)
- **Phase 2**: Build contracts (10 minutes)
- **Phase 3**: Deploy P2P contract (10 minutes)
- **Phase 4**: Deploy multi-pool contract (10 minutes)
- **Phase 5**: Configure frontend (10 minutes)
- **Phase 6**: Test contracts (20 minutes)
- **Phase 7**: End-to-end testing (15 minutes)
- **Phase 8**: Documentation (10 minutes)

## Verification Steps

1. **Environment Verification**:
   ```bash
   stellar --version
   stellar network ls
   stellar keys address deployer
   stellar keys fund deployer --network testnet
   ```

2. **Build Verification**:
   ```bash
   ls -lh contracts/target/wasm32-unknown-unknown/release/*.wasm
   file contracts/target/wasm32-unknown-unknown/release/market.wasm
   ```

3. **Deployment Verification**:
   ```bash
   stellar contract invoke --id $P2P_CONTRACT_ID --network testnet -- --help
   stellar contract invoke --id $MULTI_POOL_CONTRACT_ID --network testnet -- --help
   ```

4. **Frontend Verification**:
   - Open browser console
   - Check for contract ID logs
   - Verify no "contract not found" errors
   - Test bet creation flow

## Related Documents

- `CURRENT_FEATURES_SUMMARY.md` - Platform status
- `contracts/contracts/market/README.md` - P2P contract docs
- `contracts/contracts/multi-pool/README.md` - Multi-pool contract docs
- Stellar documentation: https://developers.stellar.org/docs/smart-contracts
