-- Cleanup migration: Ensure only PolyPulse schema exists
-- This migration ensures we're using the correct schema for tests

-- Drop any remaining old tables that might conflict
DROP TABLE IF EXISTS wager_access_log CASCADE;
DROP TABLE IF EXISTS wager_participants CASCADE;
DROP TABLE IF EXISTS wager_templates CASCADE;
DROP TABLE IF EXISTS wagers CASCADE;

-- Ensure users table has the correct schema (username, not display_name)
-- If the table exists with old schema, this will fail gracefully
-- The polypulse_schema migration should have already created the correct schema

-- Verify the users table has the expected columns
DO $$
BEGIN
    -- Check if username column exists
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'username'
    ) THEN
        RAISE EXCEPTION 'users table missing username column - polypulse_schema migration may not have run';
    END IF;
END $$;
