#!/usr/bin/env bash
# Secret Network contract deployment script
# Usage: ./scripts/deploy.sh --network <testnet|mainnet> --node <rpc_url> --chain-id <chain_id> --wasm <path>

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────────────────────────
NETWORK=""
NODE_URL=""
CHAIN_ID=""
WASM_FILE=""
KEYNAME="${SECRET_KEYNAME:-deployer}"
GAS_PRICES="0.1uscrt"
GAS_ADJUSTMENT="1.3"
ADMIN_ADDRESS="${SECRET_ADMIN_ADDRESS:-}"

# ── Argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)   NETWORK="$2";   shift 2 ;;
    --node)      NODE_URL="$2";  shift 2 ;;
    --chain-id)  CHAIN_ID="$2";  shift 2 ;;
    --wasm)      WASM_FILE="$2"; shift 2 ;;
    --key)       KEYNAME="$2";   shift 2 ;;
    --admin)     ADMIN_ADDRESS="$2"; shift 2 ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
done

# ── Validation ────────────────────────────────────────────────────────────────
if [[ -z "$NETWORK" || -z "$NODE_URL" || -z "$CHAIN_ID" || -z "$WASM_FILE" ]]; then
  echo "ERROR: --network, --node, --chain-id, and --wasm are required."
  echo "Usage: $0 --network <testnet|mainnet> --node <url> --chain-id <id> --wasm <path>"
  exit 1
fi

if [[ ! -f "$WASM_FILE" ]]; then
  echo "ERROR: Wasm file not found: $WASM_FILE"
  exit 1
fi

if ! command -v secretd &>/dev/null; then
  echo "ERROR: 'secretd' not found. Run scripts/setup-dev.sh first."
  exit 1
fi

# ── Derive admin address if not provided ─────────────────────────────────────
if [[ -z "$ADMIN_ADDRESS" ]]; then
  ADMIN_ADDRESS=$(secretd keys show "$KEYNAME" -a --keyring-backend test 2>/dev/null || true)
  if [[ -z "$ADMIN_ADDRESS" ]]; then
    echo "ERROR: Could not determine admin address. Set SECRET_ADMIN_ADDRESS or ensure key '$KEYNAME' exists."
    exit 1
  fi
fi

echo "==> Deployment configuration:"
echo "    Network   : $NETWORK"
echo "    Node      : $NODE_URL"
echo "    Chain ID  : $CHAIN_ID"
echo "    Wasm file : $WASM_FILE"
echo "    Key name  : $KEYNAME"
echo "    Admin     : $ADMIN_ADDRESS"
echo ""

# ── Upload contract ───────────────────────────────────────────────────────────
echo "==> Uploading contract wasm..."
UPLOAD_TX=$(secretd tx wasm store "$WASM_FILE" \
  --from "$KEYNAME" \
  --node "$NODE_URL" \
  --chain-id "$CHAIN_ID" \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --keyring-backend test \
  --output json \
  -y)

echo "$UPLOAD_TX" | jq .

TX_HASH=$(echo "$UPLOAD_TX" | jq -r '.txhash')
echo "==> Upload tx hash: $TX_HASH"

# Wait for tx to be included
echo "==> Waiting for transaction to be included in a block..."
sleep 6

# ── Get code ID ───────────────────────────────────────────────────────────────
CODE_ID=$(secretd query tx "$TX_HASH" \
  --node "$NODE_URL" \
  --output json | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')

if [[ -z "$CODE_ID" ]]; then
  echo "ERROR: Could not extract code_id from transaction. Check tx: $TX_HASH"
  exit 1
fi

echo "==> Contract uploaded with code_id: $CODE_ID"

# ── Instantiate contract ──────────────────────────────────────────────────────
INIT_MSG="{\"admin\":\"$ADMIN_ADDRESS\"}"
echo "==> Instantiating contract with msg: $INIT_MSG"

INIT_TX=$(secretd tx wasm instantiate "$CODE_ID" "$INIT_MSG" \
  --from "$KEYNAME" \
  --node "$NODE_URL" \
  --chain-id "$CHAIN_ID" \
  --label "wager-contract-$(date +%s)" \
  --admin "$ADMIN_ADDRESS" \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --keyring-backend test \
  --output json \
  -y)

echo "$INIT_TX" | jq .

INIT_TX_HASH=$(echo "$INIT_TX" | jq -r '.txhash')
echo "==> Instantiate tx hash: $INIT_TX_HASH"

sleep 6

CONTRACT_ADDRESS=$(secretd query tx "$INIT_TX_HASH" \
  --node "$NODE_URL" \
  --output json | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')

echo ""
echo "==> Deployment complete!"
echo "    Code ID          : $CODE_ID"
echo "    Contract Address : $CONTRACT_ADDRESS"
echo "    Network          : $NETWORK"
echo ""

# ── Save deployment info ──────────────────────────────────────────────────────
DEPLOY_DIR="$(dirname "$0")/../deployments"
mkdir -p "$DEPLOY_DIR"
DEPLOY_FILE="$DEPLOY_DIR/${NETWORK}-$(date +%Y%m%d-%H%M%S).json"

cat > "$DEPLOY_FILE" <<EOF
{
  "network": "$NETWORK",
  "chain_id": "$CHAIN_ID",
  "node": "$NODE_URL",
  "code_id": "$CODE_ID",
  "contract_address": "$CONTRACT_ADDRESS",
  "admin": "$ADMIN_ADDRESS",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "wasm_file": "$WASM_FILE"
}
EOF

echo "==> Deployment info saved to: $DEPLOY_FILE"
