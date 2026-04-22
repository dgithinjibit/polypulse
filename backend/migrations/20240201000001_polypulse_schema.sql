-- PolyPulse full schema migration
-- Replaces Django backend tables

-- Extensions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Drop old enum types if they exist from previous migrations, recreate cleanly
DO $$ BEGIN
    CREATE TYPE poll_status AS ENUM ('open','closed','resolved','cancelled','suspended');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE challenge_status AS ENUM ('pending','accepted','resolved','cancelled','expired');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE mpesa_status AS ENUM ('pending','completed','failed','cancelled');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE mpesa_tx_type AS ENUM ('deposit','withdrawal');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Drop old tables from initial migration if they exist
DROP TABLE IF EXISTS wager_access_log CASCADE;
DROP TABLE IF EXISTS wager_participants CASCADE;
DROP TABLE IF EXISTS wager_templates CASCADE;
DROP TABLE IF EXISTS wagers CASCADE;
DROP TABLE IF EXISTS notifications CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- ─── Users ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT,
    phone           TEXT,
    balance         FLOAT8 NOT NULL DEFAULT 1000.0,
    is_staff        BOOLEAN NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ─── Profiles ─────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS profiles (
    id                          BIGSERIAL PRIMARY KEY,
    user_id                     UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    avatar_url                  TEXT,
    email_verified              BOOLEAN NOT NULL DEFAULT FALSE,
    email_verification_token    UUID,
    current_streak              INT NOT NULL DEFAULT 0,
    best_streak                 INT NOT NULL DEFAULT 0,
    total_predictions           INT NOT NULL DEFAULT 0,
    correct_predictions         INT NOT NULL DEFAULT 0,
    polls_created_today         INT NOT NULL DEFAULT 0,
    last_poll_created_date      DATE,
    referral_code               TEXT UNIQUE
);

CREATE INDEX IF NOT EXISTS idx_profiles_user ON profiles(user_id);
CREATE INDEX IF NOT EXISTS idx_profiles_referral ON profiles(referral_code);

-- ─── Auth Nonces (NEAR wallet) ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS auth_nonces (
    id          BIGSERIAL PRIMARY KEY,
    account_id  TEXT NOT NULL,
    nonce       TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_nonces_account ON auth_nonces(account_id, expires_at);

-- ─── Omnichain Nonces ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS omnichain_nonces (
    id          BIGSERIAL PRIMARY KEY,
    chain       TEXT NOT NULL,
    address     TEXT NOT NULL,
    nonce       TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_omnichain_nonces_chain ON omnichain_nonces(chain, address, expires_at);

-- ─── NEAR Accounts ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS near_accounts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    account_id  TEXT NOT NULL UNIQUE,
    public_key  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_near_accounts_user ON near_accounts(user_id);

-- ─── Omnichain Accounts ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS omnichain_accounts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chain       TEXT NOT NULL,
    address     TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (chain, address)
);

CREATE INDEX IF NOT EXISTS idx_omnichain_accounts_user ON omnichain_accounts(user_id);

-- ─── User Sessions ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_sessions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_fingerprint  TEXT,
    ip_address          TEXT,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON user_sessions(user_id);

-- ─── Device Registrations ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS device_registrations (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_fingerprint  TEXT NOT NULL,
    ip_address          TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_devices_fingerprint ON device_registrations(device_fingerprint);

-- ─── Poll Categories ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS poll_categories (
    id      BIGSERIAL PRIMARY KEY,
    name    TEXT NOT NULL,
    slug    TEXT NOT NULL UNIQUE
);

INSERT INTO poll_categories (name, slug) VALUES
    ('Sports', 'sports'),
    ('Politics', 'politics'),
    ('Entertainment', 'entertainment'),
    ('Finance', 'finance'),
    ('Technology', 'technology'),
    ('Other', 'other')
ON CONFLICT (slug) DO NOTHING;

-- ─── Polls ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS polls (
    id                  BIGSERIAL PRIMARY KEY,
    creator_id          UUID NOT NULL REFERENCES users(id),
    title               TEXT NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    category_id         BIGINT REFERENCES poll_categories(id) ON DELETE SET NULL,
    status              poll_status NOT NULL DEFAULT 'open',
    is_free             BOOLEAN NOT NULL DEFAULT TRUE,
    closes_at           TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    winning_option_id   BIGINT,  -- FK added after poll_options
    resolution_criteria TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_polls_creator ON polls(creator_id);
CREATE INDEX IF NOT EXISTS idx_polls_status ON polls(status);
CREATE INDEX IF NOT EXISTS idx_polls_created ON polls(created_at DESC);

-- ─── Poll Options ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS poll_options (
    id      BIGSERIAL PRIMARY KEY,
    poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    text    TEXT NOT NULL,
    is_yes  BOOLEAN NOT NULL DEFAULT FALSE,
    "order" SMALLINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_poll_options_poll ON poll_options(poll_id);

-- Add FK for winning_option_id now that poll_options exists
ALTER TABLE polls
    ADD CONSTRAINT fk_polls_winning_option
    FOREIGN KEY (winning_option_id) REFERENCES poll_options(id)
    ON DELETE SET NULL
    NOT VALID;

-- ─── Markets ──────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS markets (
    id                  BIGSERIAL PRIMARY KEY,
    poll_id             BIGINT NOT NULL UNIQUE REFERENCES polls(id) ON DELETE CASCADE,
    liquidity_b         FLOAT8 NOT NULL DEFAULT 100.0,
    shares_outstanding  JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_markets_poll ON markets(poll_id);

-- ─── Market Price Snapshots ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS market_price_snapshots (
    id          BIGSERIAL PRIMARY KEY,
    market_id   BIGINT NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    yes_price   FLOAT8 NOT NULL,
    no_price    FLOAT8 NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_snapshots_market ON market_price_snapshots(market_id, created_at DESC);

-- ─── Bets ─────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bets (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id),
    poll_id     BIGINT NOT NULL REFERENCES polls(id),
    option_id   BIGINT NOT NULL REFERENCES poll_options(id),
    amount      FLOAT8 NOT NULL,
    shares      FLOAT8 NOT NULL DEFAULT 0.0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bets_user ON bets(user_id);
CREATE INDEX IF NOT EXISTS idx_bets_poll ON bets(poll_id);

-- ─── Market Positions ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS market_positions (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id),
    market_id       BIGINT NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    option_shares   JSONB NOT NULL DEFAULT '{}',
    option_spent    JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, market_id)
);

CREATE INDEX IF NOT EXISTS idx_positions_user ON market_positions(user_id);

-- ─── Poll Comments ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS poll_comments (
    id          BIGSERIAL PRIMARY KEY,
    poll_id     BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL,
    parent_id   BIGINT REFERENCES poll_comments(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_comments_poll ON poll_comments(poll_id);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON poll_comments(parent_id);

-- ─── Comment Likes ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS comment_likes (
    id          BIGSERIAL PRIMARY KEY,
    comment_id  BIGINT NOT NULL REFERENCES poll_comments(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (comment_id, user_id)
);

-- ─── Notifications ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS notifications (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_id            UUID REFERENCES users(id) ON DELETE SET NULL,
    notification_type   TEXT NOT NULL,
    message             TEXT NOT NULL,
    is_read             BOOLEAN NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id, is_read);

-- ─── Challenges ───────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS challenges (
    id                  BIGSERIAL PRIMARY KEY,
    creator_id          UUID NOT NULL REFERENCES users(id),
    opponent_id         UUID REFERENCES users(id),
    question            TEXT NOT NULL,
    amount              DECIMAL(10,2) NOT NULL,
    creator_choice      TEXT NOT NULL,
    status              challenge_status NOT NULL DEFAULT 'pending',
    is_open             BOOLEAN NOT NULL DEFAULT FALSE,
    poll_id             BIGINT REFERENCES polls(id) ON DELETE SET NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    resolved_at         TIMESTAMPTZ,
    winner_id           UUID REFERENCES users(id),
    resolution_criteria TEXT NOT NULL DEFAULT '',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_challenges_creator ON challenges(creator_id);
CREATE INDEX IF NOT EXISTS idx_challenges_opponent ON challenges(opponent_id);
CREATE INDEX IF NOT EXISTS idx_challenges_status ON challenges(status);

-- ─── Wallet Transactions ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id),
    amount              FLOAT8 NOT NULL,
    transaction_type    TEXT NOT NULL,
    balance_after       FLOAT8 NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    related_poll_id     BIGINT REFERENCES polls(id) ON DELETE SET NULL,
    related_bet_id      BIGINT REFERENCES bets(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wallet_tx_user ON wallet_transactions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_wallet_tx_type ON wallet_transactions(transaction_type);

-- ─── M-Pesa Transactions ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS mpesa_transactions (
    id                      BIGSERIAL PRIMARY KEY,
    user_id                 UUID NOT NULL REFERENCES users(id),
    transaction_type        mpesa_tx_type NOT NULL DEFAULT 'deposit',
    phone                   TEXT NOT NULL,
    amount                  INT NOT NULL,
    checkout_request_id     TEXT NOT NULL DEFAULT '',
    merchant_request_id     TEXT NOT NULL DEFAULT '',
    mpesa_receipt           TEXT NOT NULL DEFAULT '',
    status                  mpesa_status NOT NULL DEFAULT 'pending',
    result_desc             TEXT NOT NULL DEFAULT '',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mpesa_user ON mpesa_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_mpesa_checkout ON mpesa_transactions(checkout_request_id);
