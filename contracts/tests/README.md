# Soroban Contract Tests

This directory contains integration tests and property-based tests for the PolyPulse Soroban contracts.

## Test Structure

- `market_tests.rs` - Integration tests for the Market contract
- `challenge_tests.rs` - Integration tests for the Challenge contract
- `properties/` - Property-based tests using proptest

## Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific contract
cargo test --package market-contract
cargo test --package challenge-contract

# Run property-based tests with more iterations
PROPTEST_CASES=1000 cargo test

# Run tests with output
cargo test -- --nocapture
```

## Property-Based Testing

We use `proptest` for property-based testing to verify correctness properties across a wide range of inputs. Property tests are configured to run a minimum of 100 iterations per test.

See the design document for the complete list of correctness properties being tested.
