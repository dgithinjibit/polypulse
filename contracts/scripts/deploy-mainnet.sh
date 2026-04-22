#!/bin/bash
# Deploy contracts to Stellar mainnet
# WARNING: This deploys to production. Use with caution!

set -e

echo "WARNING: You are about to deploy to Stellar MAINNET"
echo "This will use real XLM and deploy to production."
read -p "Are you sure you want to continue? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Deployment cancelled."
    exit 0
fi

echo "Deploying contracts to Stellar mainnet..."

# Check if soroban CLI is installed
if ! command -v soroban &> /dev/null; then
    echo "Error: soroban CLI is not installed"
    echo "Install it with: cargo install --locked soroban-cli --features opt"
    exit 1
fi

# Build contracts first
./scripts/build.sh

# Configure mainnet network
echo "Configuring mainnet network..."
soroban network add \
  --global mainnet \
  --rpc-url https://soroban-rpc.mainnet.stellar.gateway.fm:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015" || true

# Deploy market contract
echo "Deploying market contract..."
MARKET_CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/market_contract.wasm \
  --source-account default \
  --network mainnet)

echo "Market contract deployed: $MARKET_CONTRACT_ID"

# Deploy challenge contract
echo "Deploying challenge contract..."
CHALLENGE_CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/challenge_contract.wasm \
  --source-account default \
  --network mainnet)

echo "Challenge contract deployed: $CHALLENGE_CONTRACT_ID"

# Save contract IDs to file
echo "Saving contract IDs..."
cat > .contract-ids-mainnet.json <<EOF
{
  "network": "mainnet",
  "marketContractId": "$MARKET_CONTRACT_ID",
  "challengeContractId": "$CHALLENGE_CONTRACT_ID",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo ""
echo "Deployment complete!"
echo "Contract IDs saved to .contract-ids-mainnet.json"
echo ""
echo "IMPORTANT: Update frontend configuration with these contract IDs"
