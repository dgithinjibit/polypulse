#!/bin/bash
# Run all tests including property-based tests

set -e

echo "Running Soroban contract tests..."

# Run unit tests
echo "Running unit tests..."
cargo test --workspace

# Run property-based tests with increased iterations
echo ""
echo "Running property-based tests (100+ iterations)..."
PROPTEST_CASES=100 cargo test --workspace -- --test-threads=1

echo ""
echo "All tests passed!"
