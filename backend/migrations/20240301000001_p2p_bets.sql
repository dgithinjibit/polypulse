-- P2P Bets table
CREATE TABLE p2p_bets (
    id BIGSERIAL PRIMARY KEY,
    creator_id BIGINT NOT NULL REFERENCES users(id),
    question TEXT NOT NULL,
    question_normalized TEXT NOT NULL,
    question_slug VARCHAR(50) NOT NULL,
    stake_amount BIGINT NOT NULL,
    end_time TIMESTAMP NOT NULL,
    state VARCHAR(20) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    shareable_url_hash VARCHAR(255) NOT NULL UNIQUE,
    contract_bet_id BIGINT,
    verified_outcome BOOLEAN,
    disputed BOOLEAN DEFAULT FALSE,
    paid_out BOOLEAN DEFAULT FALSE,
    CONSTRAINT valid_stake CHECK (stake_amount > 0),
    CONSTRAINT valid_end_time CHECK (end_time > created_at)
);

CREATE INDEX idx_p2p_bets_creator ON p2p_bets(creator_id);
CREATE INDEX idx_p2p_bets_state ON p2p_bets(state);
CREATE INDEX idx_p2p_bets_end_time ON p2p_bets(end_time);
CREATE INDEX idx_p2p_bets_url_hash ON p2p_bets(shareable_url_hash);

-- Participants table
CREATE TABLE p2p_bet_participants (
    id BIGSERIAL PRIMARY KEY,
    bet_id BIGINT NOT NULL REFERENCES p2p_bets(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id),
    position BOOLEAN NOT NULL,
    stake BIGINT NOT NULL,
    joined_at TIMESTAMP NOT NULL DEFAULT NOW(),
    has_reported BOOLEAN DEFAULT FALSE,
    UNIQUE(bet_id, user_id),
    CONSTRAINT valid_participant_stake CHECK (stake > 0)
);

CREATE INDEX idx_participants_bet ON p2p_bet_participants(bet_id);
CREATE INDEX idx_participants_user ON p2p_bet_participants(user_id);

-- Outcome reports table
CREATE TABLE p2p_outcome_reports (
    id BIGSERIAL PRIMARY KEY,
    bet_id BIGINT NOT NULL REFERENCES p2p_bets(id) ON DELETE CASCADE,
    reporter_id BIGINT NOT NULL REFERENCES users(id),
    outcome BOOLEAN NOT NULL,
    reported_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(bet_id, reporter_id)
);

CREATE INDEX idx_outcome_reports_bet ON p2p_outcome_reports(bet_id);

-- Disputes table
CREATE TABLE p2p_bet_disputes (
    id BIGSERIAL PRIMARY KEY,
    bet_id BIGINT NOT NULL REFERENCES p2p_bets(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMP,
    resolution_outcome BOOLEAN,
    resolved_by_admin_id BIGINT REFERENCES users(id),
    notes TEXT
);

CREATE INDEX idx_disputes_bet ON p2p_bet_disputes(bet_id);
