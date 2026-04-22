#!/usr/bin/env bash
# Development environment setup for Secret Network CosmWasm contracts
# Installs: Rust wasm32 target, cargo-generate, binaryen (wasm-opt), secretd

set -euo pipefail

SECRETD_VERSION="${SECRETD_VERSION:-v1.12.1}"
BINARYEN_VERSION="${BINARYEN_VERSION:-version_116}"

info()    { echo "[INFO]  $*"; }
success() { echo "[OK]    $*"; }
warn()    { echo "[WARN]  $*"; }
error()   { echo "[ERROR] $*" >&2; exit 1; }

# ── Detect OS ─────────────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"
info "Detected OS: $OS / $ARCH"

# ── Rust & wasm32 target ──────────────────────────────────────────────────────
if ! command -v rustup &>/dev/null; then
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  source "$HOME/.cargo/env"
else
  success "rustup already installed: $(rustup --version)"
fi

info "Adding wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown
success "wasm32-unknown-unknown target ready."

info "Ensuring stable toolchain components..."
rustup component add rustfmt clippy rust-src
success "Rust components ready."

# ── cargo-generate ────────────────────────────────────────────────────────────
if ! command -v cargo-generate &>/dev/null; then
  info "Installing cargo-generate..."
  cargo install cargo-generate --locked
  success "cargo-generate installed."
else
  success "cargo-generate already installed: $(cargo generate --version)"
fi

# ── cargo-run-script / cargo-make (optional, for Makefile-style tasks) ────────
if ! command -v cargo-make &>/dev/null; then
  info "Installing cargo-make..."
  cargo install cargo-make --locked
  success "cargo-make installed."
else
  success "cargo-make already installed."
fi

# ── wasm-opt (binaryen) ───────────────────────────────────────────────────────
if ! command -v wasm-opt &>/dev/null; then
  info "Installing wasm-opt (binaryen $BINARYEN_VERSION)..."
  if [[ "$OS" == "Darwin" ]]; then
    if command -v brew &>/dev/null; then
      brew install binaryen
    else
      warn "Homebrew not found. Install binaryen manually: https://github.com/WebAssembly/binaryen/releases"
    fi
  elif [[ "$OS" == "Linux" ]]; then
    TMP=$(mktemp -d)
    BINARYEN_ARCH="x86_64"
    [[ "$ARCH" == "aarch64" ]] && BINARYEN_ARCH="aarch64"
    BINARYEN_URL="https://github.com/WebAssembly/binaryen/releases/download/${BINARYEN_VERSION}/binaryen-${BINARYEN_VERSION}-${BINARYEN_ARCH}-linux.tar.gz"
    info "Downloading binaryen from $BINARYEN_URL..."
    curl -sSL "$BINARYEN_URL" -o "$TMP/binaryen.tar.gz"
    tar -xzf "$TMP/binaryen.tar.gz" -C "$TMP"
    sudo cp "$TMP/binaryen-${BINARYEN_VERSION}/bin/wasm-opt" /usr/local/bin/
    rm -rf "$TMP"
    success "wasm-opt installed."
  else
    warn "Unsupported OS for automatic wasm-opt install. Install binaryen manually."
  fi
else
  success "wasm-opt already installed: $(wasm-opt --version)"
fi

# ── Docker (required for cosmwasm-optimizer) ──────────────────────────────────
if ! command -v docker &>/dev/null; then
  warn "Docker not found. The 'make optimize' target requires Docker."
  warn "Install Docker from: https://docs.docker.com/get-docker/"
else
  success "Docker available: $(docker --version)"
fi

# ── secretd CLI ───────────────────────────────────────────────────────────────
if ! command -v secretd &>/dev/null; then
  info "Installing secretd $SECRETD_VERSION..."
  if [[ "$OS" == "Darwin" ]]; then
    if command -v brew &>/dev/null; then
      brew install scrtlabs/tap/secretd
    else
      warn "Homebrew not found. Install secretd manually from: https://github.com/scrtlabs/SecretNetwork/releases"
    fi
  elif [[ "$OS" == "Linux" ]]; then
    SECRETD_ARCH="amd64"
    [[ "$ARCH" == "aarch64" ]] && SECRETD_ARCH="arm64"
    SECRETD_URL="https://github.com/scrtlabs/SecretNetwork/releases/download/${SECRETD_VERSION}/secretd-${SECRETD_VERSION}-linux-${SECRETD_ARCH}"
    info "Downloading secretd from $SECRETD_URL..."
    curl -sSL "$SECRETD_URL" -o /tmp/secretd
    chmod +x /tmp/secretd
    sudo mv /tmp/secretd /usr/local/bin/secretd
    success "secretd $SECRETD_VERSION installed."
  else
    warn "Unsupported OS for automatic secretd install. Download from: https://github.com/scrtlabs/SecretNetwork/releases"
  fi
else
  success "secretd already installed: $(secretd version 2>/dev/null || echo 'unknown version')"
fi

# ── jq (used by deploy.sh) ────────────────────────────────────────────────────
if ! command -v jq &>/dev/null; then
  info "Installing jq..."
  if [[ "$OS" == "Darwin" ]]; then
    brew install jq
  elif [[ "$OS" == "Linux" ]]; then
    sudo apt-get install -y jq 2>/dev/null || sudo yum install -y jq 2>/dev/null || warn "Could not install jq automatically."
  fi
else
  success "jq already installed: $(jq --version)"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "============================================"
echo " Development environment setup complete!"
echo "============================================"
echo ""
echo "Next steps:"
echo "  1. cd rust/contracts"
echo "  2. make build          # compile for wasm32"
echo "  3. make test           # run unit tests"
echo "  4. make optimize       # produce optimized wasm (requires Docker)"
echo "  5. make deploy-testnet # deploy to Secret Network testnet"
echo ""
echo "Set these env vars before deploying:"
echo "  SECRET_KEYNAME        - secretd key name (default: deployer)"
echo "  SECRET_ADMIN_ADDRESS  - contract admin address"
echo ""
