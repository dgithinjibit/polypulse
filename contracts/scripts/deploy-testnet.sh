#!/bin/bash
# Deploy contracts to Stellar testnet

set -e

echo "Deploying contracts to Stellar testnet..."

# Check if soroban CLI is installed
if ! command -v soroban &> /dev/null; then
    echo "Error: soroban CLI is not installed"
    echo "Install it with: cargo install --locked soroban-cli --features opt"
    exit 1
fi

# Build contracts first
./scripts/build.sh

# Configure testnet network
echo "Configuring testnet network..."
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015" || true

# Deploy market contract
echo "Deploying market contract..."
MARKET_CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market_contract.wasm \
  --source-account default \
  --network testnet)

echo "Market contract deployed: $MARKET_CONTRACT_ID"

# Deploy challenge contract
echo "Deploying challenge contract..."
CHALLENGE_CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/challenge_contract.wasm \
  --source-account default \
  --network testnet)

echo "Challenge contract deployed: $CHALLENGE_CONTRACT_ID"

# Save contract IDs to file
echo "Saving contract IDs..."
cat > .contract-ids-testnet.json <<EOF
{
  "network": "testnet",
  "marketContractId": "$MARKET_CONTRACT_ID",
  "challengeContractId": "$CHALLENGE_CONTRACT_ID",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo ""
echo "Deployment complete!"
echo "Contract IDs saved to .contract-ids-testnet.json"
