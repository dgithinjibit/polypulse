-- Add missing columns to wallet_transactions table
-- These columns are required by the refactored wallet_transactions service

-- Add balance_before column (tracks balance before transaction)
ALTER TABLE wallet_transactions 
ADD COLUMN IF NOT EXISTS balance_before BIGINT;

-- Add reference_id column (generic reference to related entities)
ALTER TABLE wallet_transactions 
ADD COLUMN IF NOT EXISTS reference_id UUID;

-- Backfill balance_before for existing records
-- For existing records, we'll calculate balance_before from balance_after - amount
UPDATE wallet_transactions 
SET balance_before = balance_after - amount::BIGINT
WHERE balance_before IS NULL;

-- Make balance_before NOT NULL after backfill
ALTER TABLE wallet_transactions 
ALTER COLUMN balance_before SET NOT NULL;

-- Migrate existing related_bet_id and related_poll_id to reference_id
-- Priority: bet_id > poll_id (if both exist, use bet_id)
UPDATE wallet_transactions 
SET reference_id = (
    CASE 
        WHEN related_bet_id IS NOT NULL THEN related_bet_id::TEXT::UUID
        WHEN related_poll_id IS NOT NULL THEN related_poll_id::TEXT::UUID
        ELSE NULL
    END
)
WHERE reference_id IS NULL;

-- Add index on reference_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_wallet_tx_reference ON wallet_transactions(reference_id);
