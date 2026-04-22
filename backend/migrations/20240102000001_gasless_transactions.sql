-- Gasless transactions table for tracking paymaster-sponsored transactions

CREATE TABLE IF NOT EXISTS gasless_transactions (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id),
    tx_hash         TEXT,
    status          TEXT NOT NULL,
    retry_count     INT NOT NULL DEFAULT 0,
    gas_used        BIGINT,
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gasless_tx_user ON gasless_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_gasless_tx_status ON gasless_transactions(status);
CREATE INDEX IF NOT EXISTS idx_gasless_tx_created ON gasless_transactions(created_at DESC);
