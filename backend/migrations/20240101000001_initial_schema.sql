-- Initial schema for PolyPulse Rust backend
-- Runs alongside the existing Django schema on the same Postgres instance.
-- All tables are prefixed with `pp_` to avoid collisions.

-- Users table (mirrors Django auth_user but for Rust backend)
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    wallet_address  TEXT,
    balance         BIGINT NOT NULL DEFAULT 1000,
    referral_code   TEXT NOT NULL UNIQUE,
    reputation_score INT NOT NULL DEFAULT 100,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Wager status enum
DO $$ BEGIN
    CREATE TYPE wager_status AS ENUM ('pending', 'active', 'resolved', 'cancelled', 'expired');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Resolution type enum
DO $$ BEGIN
    CREATE TYPE resolution_type AS ENUM ('ai_oracle', 'trusted_judge', 'social_consensus');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Wagers table
CREATE TABLE IF NOT EXISTS wagers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    creator_id          UUID NOT NULL REFERENCES users(id),
    description         TEXT NOT NULL,
    resolution_criteria TEXT NOT NULL,
    amount              BIGINT NOT NULL,
    max_participants    INT NOT NULL DEFAULT 2,
    status              wager_status NOT NULL DEFAULT 'pending',
    resolution_type     resolution_type,
    trusted_judge_id    UUID REFERENCES users(id),
    is_public           BOOLEAN NOT NULL DEFAULT FALSE,
    winner_id           UUID REFERENCES users(id),
    expires_at          TIMESTAMPTZ NOT NULL,
    resolved_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wagers_creator ON wagers(creator_id);
CREATE INDEX IF NOT EXISTS idx_wagers_status ON wagers(status);
CREATE INDEX IF NOT EXISTS idx_wagers_expires_at ON wagers(expires_at);

-- Wager participants
CREATE TABLE IF NOT EXISTS wager_participants (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wager_id    UUID NOT NULL REFERENCES wagers(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id),
    amount      BIGINT NOT NULL,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (wager_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_wager_participants_user ON wager_participants(user_id);
CREATE INDEX IF NOT EXISTS idx_wager_participants_wager ON wager_participants(wager_id);

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id),
    event_type  TEXT NOT NULL,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    is_read     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id, is_read);

-- Wager access log (for security auditing without revealing identities)
CREATE TABLE IF NOT EXISTS wager_access_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wager_id    UUID NOT NULL REFERENCES wagers(id),
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    -- Intentionally no user_id to preserve privacy (req 2.5)
);

-- Wager templates
CREATE TABLE IF NOT EXISTS wager_templates (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                TEXT NOT NULL,
    category            TEXT NOT NULL,
    description_template TEXT NOT NULL,
    criteria_template   TEXT NOT NULL,
    usage_count         INT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default templates
INSERT INTO wager_templates (id, name, category, description_template, criteria_template)
VALUES
    (gen_random_uuid(), 'Sports Game Winner', 'sports', 'Who will win {team_a} vs {team_b}?', 'The winner is determined by the final score of the game on {date}.'),
    (gen_random_uuid(), 'Project Deadline', 'work', 'Will {person} finish {project} by {date}?', 'The project is considered complete when {completion_criteria} by {date}.'),
    (gen_random_uuid(), 'Personal Goal', 'personal', 'Will {person} achieve {goal} by {date}?', '{person} must demonstrate {evidence} by {date}.'),
    (gen_random_uuid(), 'Movie Box Office', 'entertainment', 'Will {movie} gross over {amount} in its opening weekend?', 'Based on official box office reports published by {date}.'),
    (gen_random_uuid(), 'Weather Bet', 'personal', 'Will it rain in {city} on {date}?', 'Based on official weather station data for {city} on {date}.'),
    (gen_random_uuid(), 'Election Outcome', 'politics', 'Who will win the {election} election?', 'Based on official certified results published after {date}.'),
    (gen_random_uuid(), 'Stock Price', 'finance', 'Will {ticker} be above {price} on {date}?', 'Based on closing price of {ticker} on {date} per official exchange data.'),
    (gen_random_uuid(), 'Fitness Challenge', 'personal', 'Will {person} run {distance} by {date}?', '{person} must provide verified fitness tracker data showing {distance} completed by {date}.'),
    (gen_random_uuid(), 'Game Score', 'entertainment', 'Will {person} reach level {level} in {game} by {date}?', '{person} must share a screenshot of their in-game progress showing level {level} by {date}.'),
    (gen_random_uuid(), 'Custom Event', 'other', '{custom_description}', '{custom_criteria}')
ON CONFLICT DO NOTHING;
