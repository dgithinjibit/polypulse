-- Migration: Cleanup Schema - Drop old tables and rename pp_ prefixed tables
-- This migration removes duplicate old tables and renames the PolyPulse schema tables
-- to match the code models (removing pp_ prefix)

BEGIN;

-- Step 1: Drop old tables that are no longer used
-- These tables are from the old schema and have been replaced by pp_ prefixed tables

-- Drop wager-related tables (old system)
DROP TABLE IF EXISTS wager_access_log CASCADE;
DROP TABLE IF EXISTS wager_participants CASCADE;
DROP TABLE IF EXISTS wager_templates CASCADE;
DROP TABLE IF EXISTS wagers CASCADE;

-- Drop old gasless transactions table
DROP TABLE IF EXISTS gasless_transactions CASCADE;

-- Drop old wallet_transactions table (replaced by pp_wallet_transactions)
DROP TABLE IF EXISTS wallet_transactions CASCADE;

-- Drop old notifications table (replaced by pp_notifications)
DROP TABLE IF EXISTS notifications CASCADE;

-- Drop old users table (replaced by pp_users)
DROP TABLE IF EXISTS users CASCADE;

-- Step 2: Rename pp_ prefixed tables to remove the prefix
-- This makes the schema match the code models

-- Core user tables
ALTER TABLE pp_users RENAME TO users;
ALTER TABLE pp_profiles RENAME TO profiles;
ALTER TABLE pp_user_sessions RENAME TO user_sessions;

-- Authentication tables
ALTER TABLE pp_auth_nonces RENAME TO auth_nonces;
ALTER TABLE pp_near_accounts RENAME TO near_accounts;
ALTER TABLE pp_omnichain_accounts RENAME TO omnichain_accounts;
ALTER TABLE pp_omnichain_nonces RENAME TO omnichain_nonces;

-- Poll and market tables
ALTER TABLE pp_polls RENAME TO polls;
ALTER TABLE pp_poll_options RENAME TO poll_options;
ALTER TABLE pp_poll_categories RENAME TO poll_categories;
ALTER TABLE pp_poll_comments RENAME TO poll_comments;
ALTER TABLE pp_comment_likes RENAME TO comment_likes;

-- Market and betting tables
ALTER TABLE pp_markets RENAME TO markets;
ALTER TABLE pp_bets RENAME TO bets;
ALTER TABLE pp_market_positions RENAME TO market_positions;
ALTER TABLE pp_market_price_snapshots RENAME TO market_price_snapshots;

-- Challenge tables
ALTER TABLE pp_challenges RENAME TO challenges;

-- Wallet tables
ALTER TABLE pp_wallet_transactions RENAME TO wallet_transactions;
ALTER TABLE pp_mpesa_transactions RENAME TO mpesa_transactions;

-- Notification tables
ALTER TABLE pp_notifications RENAME TO notifications;

-- Device registration tables
ALTER TABLE pp_device_registrations RENAME TO device_registrations;

-- Step 3: Update sequence names to match new table names
-- Sequences are automatically created with table_column_seq naming

ALTER SEQUENCE IF EXISTS pp_profiles_id_seq RENAME TO profiles_id_seq;
ALTER SEQUENCE IF EXISTS pp_user_sessions_id_seq RENAME TO user_sessions_id_seq;
ALTER SEQUENCE IF EXISTS pp_auth_nonces_id_seq RENAME TO auth_nonces_id_seq;
ALTER SEQUENCE IF EXISTS pp_near_accounts_id_seq RENAME TO near_accounts_id_seq;
ALTER SEQUENCE IF EXISTS pp_omnichain_accounts_id_seq RENAME TO omnichain_accounts_id_seq;
ALTER SEQUENCE IF EXISTS pp_omnichain_nonces_id_seq RENAME TO omnichain_nonces_id_seq;
ALTER SEQUENCE IF EXISTS pp_polls_id_seq RENAME TO polls_id_seq;
ALTER SEQUENCE IF EXISTS pp_poll_options_id_seq RENAME TO poll_options_id_seq;
ALTER SEQUENCE IF EXISTS pp_poll_categories_id_seq RENAME TO poll_categories_id_seq;
ALTER SEQUENCE IF EXISTS pp_poll_comments_id_seq RENAME TO poll_comments_id_seq;
ALTER SEQUENCE IF EXISTS pp_comment_likes_id_seq RENAME TO comment_likes_id_seq;
ALTER SEQUENCE IF EXISTS pp_markets_id_seq RENAME TO markets_id_seq;
ALTER SEQUENCE IF EXISTS pp_bets_id_seq RENAME TO bets_id_seq;
ALTER SEQUENCE IF EXISTS pp_market_positions_id_seq RENAME TO market_positions_id_seq;
ALTER SEQUENCE IF EXISTS pp_market_price_snapshots_id_seq RENAME TO market_price_snapshots_id_seq;
ALTER SEQUENCE IF EXISTS pp_challenges_id_seq RENAME TO challenges_id_seq;
ALTER SEQUENCE IF EXISTS pp_wallet_transactions_id_seq RENAME TO wallet_transactions_id_seq;
ALTER SEQUENCE IF EXISTS pp_mpesa_transactions_id_seq RENAME TO mpesa_transactions_id_seq;
ALTER SEQUENCE IF EXISTS pp_notifications_id_seq RENAME TO notifications_id_seq;
ALTER SEQUENCE IF EXISTS pp_device_registrations_id_seq RENAME TO device_registrations_id_seq;

-- Step 4: Update index names to match new table names
-- This is optional but keeps naming consistent

-- Users table indexes
ALTER INDEX IF EXISTS pp_users_pkey RENAME TO users_pkey;
ALTER INDEX IF EXISTS pp_users_email_key RENAME TO users_email_key;
ALTER INDEX IF EXISTS pp_users_username_key RENAME TO users_username_key;

-- Profiles table indexes
ALTER INDEX IF EXISTS pp_profiles_pkey RENAME TO profiles_pkey;
ALTER INDEX IF EXISTS pp_profiles_user_id_key RENAME TO profiles_user_id_key;
ALTER INDEX IF EXISTS pp_profiles_referral_code_key RENAME TO profiles_referral_code_key;

-- User sessions table indexes
ALTER INDEX IF EXISTS pp_user_sessions_pkey RENAME TO user_sessions_pkey;
ALTER INDEX IF EXISTS pp_user_sessions_user_id_idx RENAME TO user_sessions_user_id_idx;
ALTER INDEX IF EXISTS pp_user_sessions_refresh_token_key RENAME TO user_sessions_refresh_token_key;

-- Auth nonces table indexes
ALTER INDEX IF EXISTS pp_auth_nonces_pkey RENAME TO auth_nonces_pkey;
ALTER INDEX IF EXISTS pp_auth_nonces_nonce_key RENAME TO auth_nonces_key;

-- Near accounts table indexes
ALTER INDEX IF EXISTS pp_near_accounts_pkey RENAME TO near_accounts_pkey;
ALTER INDEX IF EXISTS pp_near_accounts_user_id_key RENAME TO near_accounts_user_id_key;
ALTER INDEX IF EXISTS pp_near_accounts_account_id_key RENAME TO near_accounts_account_id_key;

-- Omnichain accounts table indexes
ALTER INDEX IF EXISTS pp_omnichain_accounts_pkey RENAME TO omnichain_accounts_pkey;
ALTER INDEX IF EXISTS pp_omnichain_accounts_user_id_idx RENAME TO omnichain_accounts_user_id_idx;

-- Omnichain nonces table indexes
ALTER INDEX IF EXISTS pp_omnichain_nonces_pkey RENAME TO omnichain_nonces_pkey;
ALTER INDEX IF EXISTS pp_omnichain_nonces_nonce_key RENAME TO omnichain_nonces_nonce_key;

-- Polls table indexes
ALTER INDEX IF EXISTS pp_polls_pkey RENAME TO polls_pkey;
ALTER INDEX IF EXISTS pp_polls_creator_id_idx RENAME TO polls_creator_id_idx;
ALTER INDEX IF EXISTS pp_polls_status_idx RENAME TO polls_status_idx;
ALTER INDEX IF EXISTS pp_polls_category_id_idx RENAME TO polls_category_id_idx;

-- Poll options table indexes
ALTER INDEX IF EXISTS pp_poll_options_pkey RENAME TO poll_options_pkey;
ALTER INDEX IF EXISTS pp_poll_options_poll_id_idx RENAME TO poll_options_poll_id_idx;

-- Poll categories table indexes
ALTER INDEX IF EXISTS pp_poll_categories_pkey RENAME TO poll_categories_pkey;
ALTER INDEX IF EXISTS pp_poll_categories_name_key RENAME TO poll_categories_name_key;

-- Poll comments table indexes
ALTER INDEX IF EXISTS pp_poll_comments_pkey RENAME TO poll_comments_pkey;
ALTER INDEX IF EXISTS pp_poll_comments_poll_id_idx RENAME TO poll_comments_poll_id_idx;
ALTER INDEX IF EXISTS pp_poll_comments_user_id_idx RENAME TO poll_comments_user_id_idx;
ALTER INDEX IF EXISTS pp_poll_comments_parent_id_idx RENAME TO poll_comments_parent_id_idx;

-- Comment likes table indexes
ALTER INDEX IF EXISTS pp_comment_likes_pkey RENAME TO comment_likes_pkey;
ALTER INDEX IF EXISTS pp_comment_likes_comment_id_user_id_key RENAME TO comment_likes_comment_id_user_id_key;

-- Markets table indexes
ALTER INDEX IF EXISTS pp_markets_pkey RENAME TO markets_pkey;
ALTER INDEX IF EXISTS pp_markets_poll_id_key RENAME TO markets_poll_id_key;

-- Bets table indexes
ALTER INDEX IF EXISTS pp_bets_pkey RENAME TO bets_pkey;
ALTER INDEX IF EXISTS pp_bets_user_id_idx RENAME TO bets_user_id_idx;
ALTER INDEX IF EXISTS pp_bets_poll_id_idx RENAME TO bets_poll_id_idx;
ALTER INDEX IF EXISTS pp_bets_option_id_idx RENAME TO bets_option_id_idx;

-- Market positions table indexes
ALTER INDEX IF EXISTS pp_market_positions_pkey RENAME TO market_positions_pkey;
ALTER INDEX IF EXISTS pp_market_positions_user_id_market_id_key RENAME TO market_positions_user_id_market_id_key;

-- Market price snapshots table indexes
ALTER INDEX IF EXISTS pp_market_price_snapshots_pkey RENAME TO market_price_snapshots_pkey;
ALTER INDEX IF EXISTS pp_market_price_snapshots_market_id_idx RENAME TO market_price_snapshots_market_id_idx;

-- Challenges table indexes
ALTER INDEX IF EXISTS pp_challenges_pkey RENAME TO challenges_pkey;
ALTER INDEX IF EXISTS pp_challenges_creator_id_idx RENAME TO challenges_creator_id_idx;
ALTER INDEX IF EXISTS pp_challenges_opponent_id_idx RENAME TO challenges_opponent_id_idx;
ALTER INDEX IF EXISTS pp_challenges_poll_id_idx RENAME TO challenges_poll_id_idx;

-- Wallet transactions table indexes
ALTER INDEX IF EXISTS pp_wallet_transactions_pkey RENAME TO wallet_transactions_pkey;
ALTER INDEX IF EXISTS pp_wallet_transactions_user_id_idx RENAME TO wallet_transactions_user_id_idx;

-- Mpesa transactions table indexes
ALTER INDEX IF EXISTS pp_mpesa_transactions_pkey RENAME TO mpesa_transactions_pkey;
ALTER INDEX IF EXISTS pp_mpesa_transactions_user_id_idx RENAME TO mpesa_transactions_user_id_idx;
ALTER INDEX IF EXISTS pp_mpesa_transactions_checkout_request_id_key RENAME TO mpesa_transactions_checkout_request_id_key;

-- Notifications table indexes
ALTER INDEX IF EXISTS pp_notifications_pkey RENAME TO notifications_pkey;
ALTER INDEX IF EXISTS pp_notifications_user_id_idx RENAME TO notifications_user_id_idx;

-- Device registrations table indexes
ALTER INDEX IF EXISTS pp_device_registrations_pkey RENAME TO device_registrations_pkey;
ALTER INDEX IF EXISTS pp_device_registrations_user_id_idx RENAME TO device_registrations_user_id_idx;
ALTER INDEX IF EXISTS pp_device_registrations_token_key RENAME TO device_registrations_token_key;

COMMIT;
