#!/bin/bash
# Build script for Soroban contracts

set -e

echo "Building Soroban contracts..."

# Build market contract
echo "Building market contract..."
cd contracts/market
cargo build --target wasm32-unknown-unknown --release
cd ../..

# Build challenge contract
echo "Building challenge contract..."
cd contracts/challenge
cargo build --target wasm32-unknown-unknown --release
cd ../..

echo "Build complete!"
echo ""
echo "Contract WASMs are located at:"
echo "  - target/wasm32-unknown-unknown/release/market_contract.wasm"
echo "  - target/wasm32-unknown-unknown/release/challenge_contract.wasm"
