# PolyPulse Soroban Contracts

This directory contains the Soroban smart contracts for the PolyPulse prediction markets platform on Stellar.

## Overview

The PolyPulse platform is migrating from a centralized Django/PostgreSQL architecture to a fully decentralized system powered by Soroban smart contracts. This implementation includes:

- **Market Contract**: Manages prediction markets with LMSR pricing
- **Challenge Contract**: Handles head-to-head prediction challenges

## Prerequisites

### Required

- Rust 1.79.0 or higher (recommended for Soroban SDK compatibility)
- Cargo (comes with Rust)

**Note**: The project currently has Rust 1.75.0 installed. To use the latest Soroban SDK features, you'll need to update Rust:
```bash
rustup update
```

Alternatively, the project can work with Rust 1.75.0 by using older Soroban SDK versions (20.x), but this may limit access to newer features.

### Installation

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Soroban CLI**:
   ```bash
   cargo install --locked soroban-cli --features opt
   ```
   
   Note: If you have Rust 1.75.0, install a compatible version:
   ```bash
   cargo install --locked soroban-cli --version 21.1.1 --features opt
   ```

3. **Add WASM target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

## Project Structure

```
soroban/
├── contracts/
│   ├── market/          # Market contract (LMSR pricing, trading)
│   └── challenge/       # Challenge contract (head-to-head predictions)
├── tests/
│   ├── properties/      # Property-based tests using proptest
│   ├── market_tests.rs  # Market integration tests
│   └── challenge_tests.rs # Challenge integration tests
├── scripts/
│   ├── build.sh         # Build contracts
│   ├── test.sh          # Run all tests
│   ├── deploy-testnet.sh # Deploy to testnet
│   └── deploy-mainnet.sh # Deploy to mainnet
└── Cargo.toml           # Workspace configuration
```

## Development Workflow

### Building Contracts

```bash
cd soroban
./scripts/build.sh
```

This compiles both contracts to WASM and places them in `target/wasm32-unknown-unknown/release/`.

### Running Tests

```bash
# Run all tests
./scripts/test.sh

# Run tests for a specific contract
cargo test --package market-contract
cargo test --package challenge-contract

# Run with verbose output
cargo test -- --nocapture

# Run property-based tests with more iterations
PROPTEST_CASES=1000 cargo test
```

### Deploying to Testnet

```bash
# First, create a testnet account and fund it with test XLM
soroban keys generate --global default --network testnet

# Get test XLM from the friendbot
curl "https://friendbot.stellar.org?addr=$(soroban keys address default)"

# Deploy contracts
./scripts/deploy-testnet.sh
```

Contract IDs will be saved to `.contract-ids-testnet.json`.

### Deploying to Mainnet

⚠️ **WARNING**: This deploys to production and uses real XLM!

```bash
# Ensure you have a funded mainnet account configured
soroban keys generate --global mainnet-deployer --network mainnet

# Deploy (will prompt for confirmation)
./scripts/deploy-mainnet.sh
```

Contract IDs will be saved to `.contract-ids-mainnet.json`.

## Testing Strategy

The project uses a comprehensive testing approach:

### Unit Tests
- Located in each contract's `src/lib.rs` under `#[cfg(test)]`
- Test individual functions and edge cases
- Run with `cargo test`

### Integration Tests
- Located in `tests/market_tests.rs` and `tests/challenge_tests.rs`
- Test full workflows and contract interactions
- Deploy contracts to test environment

### Property-Based Tests
- Located in `tests/properties/`
- Use `proptest` to verify correctness properties across many inputs
- Minimum 100 iterations per property test
- Test LMSR pricing, state consistency, XLM conservation

See the [design document](../.kiro/specs/full-stellar-migration/design.md) for the complete list of 19 correctness properties being tested.

## Contract APIs

### Market Contract

```rust
// Create a new prediction market
create_market(creator, title, description, options, close_time, liquidity_b, resolution_criteria) -> market_id

// Buy shares in a market option
buy_shares(buyer, market_id, option_id, xlm_amount) -> BuyResult

// Sell shares from a position
sell_shares(seller, market_id, option_id, shares) -> SellResult

// Resolve a market with winning option
resolve_market(resolver, market_id, winning_option_id)

// Get current price for an option
get_price(market_id, option_id) -> price

// Get user's position in a market
get_position(user, market_id) -> Position

// Get market details
get_market(market_id) -> Market
```

### Challenge Contract

```rust
// Create a new challenge
create_challenge(creator, question, xlm_stake, creator_choice, expires_at, is_open, resolution_criteria) -> challenge_id

// Accept an existing challenge
accept_challenge(opponent, challenge_id)

// Resolve a challenge with winner
resolve_challenge(resolver, challenge_id, winner)

// Cancel an unaccepted challenge
cancel_challenge(creator, challenge_id)

// Get challenge details
get_challenge(challenge_id) -> Challenge
```

## LMSR Implementation

The Market contract implements the Logarithmic Market Scoring Rule (LMSR) for automated market making:

**Cost Function**: `C(q) = b × ln(Σ exp(q_i / b))`

Where:
- `q_i` = shares outstanding for option i
- `b` = liquidity parameter (controls price sensitivity)
- Prices always sum to 1 (valid probability distribution)

See the design document for detailed LMSR implementation notes.

## Configuration

### Workspace Dependencies

All contracts share common dependencies defined in the workspace `Cargo.toml`:
- `soroban-sdk = "21.7.0"` - Soroban SDK for contract development
- `proptest = "1.4"` - Property-based testing framework

### Build Profiles

- **release**: Optimized for deployment (small WASM size, no debug info)
- **release-with-logs**: Release build with debug assertions for testing

## Troubleshooting

### Soroban CLI Installation Issues

If you encounter Rust version compatibility issues:
```bash
# Update Rust to the latest version
rustup update

# Or install a specific compatible Soroban CLI version
cargo install --locked soroban-cli --version 21.1.1 --features opt
```

### Build Errors

If you get WASM target errors:
```bash
rustup target add wasm32-unknown-unknown
```

### Test Failures

Property-based tests may occasionally fail due to random input generation. If a test fails:
1. Note the failing seed (printed in test output)
2. Re-run with that seed to reproduce: `PROPTEST_SEED=<seed> cargo test`
3. Investigate the specific input that caused the failure

## Resources

- [Soroban Documentation](https://developers.stellar.org/docs/smart-contracts)
- [Stellar Network](https://stellar.org)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
- [Proptest Documentation](https://docs.rs/proptest/)

## License

See the main project LICENSE file.
