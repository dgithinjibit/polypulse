-- Auth nonces table for wallet-based login (Stellar, NEAR, omnichain)
CREATE TABLE IF NOT EXISTS pp_auth_nonces (
    id          BIGSERIAL PRIMARY KEY,
    account_id  TEXT NOT NULL,
    nonce       TEXT NOT NULL,
    used        BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pp_auth_nonces_account ON pp_auth_nonces(account_id);
CREATE INDEX IF NOT EXISTS idx_pp_auth_nonces_expires ON pp_auth_nonces(expires_at);

-- Omnichain accounts (Stellar, NEAR, EVM, etc.)
-- No FK to pp_users since that table is managed by Django migrations
CREATE TABLE IF NOT EXISTS pp_omnichain_accounts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL,
    chain       TEXT NOT NULL,
    address     TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain, address)
);

CREATE INDEX IF NOT EXISTS idx_pp_omnichain_user ON pp_omnichain_accounts(user_id);

-- NEAR accounts
CREATE TABLE IF NOT EXISTS pp_near_accounts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL,
    account_id  TEXT NOT NULL UNIQUE,
    public_key  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pp_near_user ON pp_near_accounts(user_id);

-- Omnichain nonces (used by omnichain-nonce endpoint)
CREATE TABLE IF NOT EXISTS pp_omnichain_nonces (
    id          BIGSERIAL PRIMARY KEY,
    chain       TEXT NOT NULL,
    address     TEXT NOT NULL,
    nonce       TEXT NOT NULL,
    used        BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pp_omnichain_nonces ON pp_omnichain_nonces(chain, address);
