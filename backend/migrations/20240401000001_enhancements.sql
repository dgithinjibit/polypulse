-- Multi-participant pool support
ALTER TABLE p2p_bets ADD COLUMN is_multi_participant BOOLEAN DEFAULT FALSE;
ALTER TABLE p2p_bets ADD COLUMN total_yes_stakes BIGINT DEFAULT 0;
ALTER TABLE p2p_bets ADD COLUMN total_no_stakes BIGINT DEFAULT 0;
ALTER TABLE p2p_bets ADD COLUMN participant_count INTEGER DEFAULT 0;

CREATE INDEX idx_p2p_bets_multi ON p2p_bets(is_multi_participant) WHERE is_multi_participant = true;
CREATE INDEX idx_p2p_bets_participant_count ON p2p_bets(participant_count DESC);

-- Telegram integration
CREATE TABLE telegram_users (
    id BIGSERIAL PRIMARY KEY,
    telegram_id BIGINT NOT NULL UNIQUE,
    telegram_username VARCHAR(255),
    user_id BIGINT REFERENCES users(id),
    wallet_address VARCHAR(56),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_telegram_users_telegram_id ON telegram_users(telegram_id);
CREATE INDEX idx_telegram_users_user_id ON telegram_users(user_id);

-- XP and gamification
ALTER TABLE users ADD COLUMN IF NOT EXISTS xp INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS level INTEGER DEFAULT 1;
ALTER TABLE users ADD COLUMN IF NOT EXISTS win_streak INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_premium BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS premium_activated_at TIMESTAMP;
ALTER TABLE users ADD COLUMN IF NOT EXISTS reputation_score INTEGER DEFAULT 50;
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_verified BOOLEAN DEFAULT FALSE;

CREATE INDEX idx_users_xp ON users(xp DESC);
CREATE INDEX idx_users_reputation ON users(reputation_score DESC);

-- Achievements
CREATE TABLE achievements (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    icon_url VARCHAR(255),
    xp_reward INTEGER DEFAULT 0
);

CREATE TABLE user_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id BIGINT NOT NULL REFERENCES achievements(id),
    unlocked_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, achievement_id)
);

CREATE INDEX idx_user_achievements_user ON user_achievements(user_id);

-- Reputation events
CREATE TABLE reputation_events (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    event_type VARCHAR(50) NOT NULL,
    score_change INTEGER NOT NULL,
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reputation_events_user ON reputation_events(user_id);

-- Insert default achievements
INSERT INTO achievements (code, name, description, xp_reward) VALUES
('first_win', 'First Win', 'Won your first bet', 50),
('10_bets', '10 Bets Created', 'Created 10 bets', 100),
('5_day_streak', '5-Day Streak', 'Correct predictions for 5 days in a row', 200),
('dispute_resolver', 'Dispute Resolver', 'Helped resolve a disputed bet', 150),
('early_adopter', 'Early Adopter', 'Joined during beta', 500);

-- PWA push subscriptions
CREATE TABLE push_subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, endpoint)
);

CREATE INDEX idx_push_subscriptions_user ON push_subscriptions(user_id);

-- Bet templates
CREATE TABLE bet_templates (
    id BIGSERIAL PRIMARY KEY,
    category VARCHAR(50) NOT NULL,
    name VARCHAR(100) NOT NULL,
    question_template TEXT NOT NULL,
    variables JSONB NOT NULL,
    suggested_end_time VARCHAR(50),
    usage_count INTEGER DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bet_templates_category ON bet_templates(category);

-- Activity feed
CREATE TABLE activities (
    id VARCHAR(36) PRIMARY KEY,
    activity_type TEXT NOT NULL,
    user_id BIGINT NOT NULL,
    username VARCHAR(255) NOT NULL,
    avatar_url VARCHAR(255),
    bet_id VARCHAR(50),
    bet_question TEXT,
    amount BIGINT,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_activities_timestamp ON activities(timestamp DESC);
CREATE INDEX idx_activities_bet_id ON activities(bet_id) WHERE bet_id IS NOT NULL;
CREATE INDEX idx_activities_user_id ON activities(user_id);

-- Insert default bet templates
INSERT INTO bet_templates (category, name, question_template, variables) VALUES
('crypto', 'Bitcoin Price', 'Will {{crypto}} reach ${{price}} by {{date}}?', '[{"name":"crypto","var_type":"crypto","required":true,"autocomplete":true},{"name":"price","var_type":"price","required":true,"autocomplete":false},{"name":"date","var_type":"date","required":true,"autocomplete":false}]'),
('sports', 'Team Victory', 'Will {{team}} win against {{opponent}} on {{date}}?', '[{"name":"team","var_type":"team","required":true,"autocomplete":true},{"name":"opponent","var_type":"team","required":true,"autocomplete":true},{"name":"date","var_type":"date","required":true,"autocomplete":false}]'),
('weather', 'Rain Prediction', 'Will it rain in {{city}} on {{date}}?', '[{"name":"city","var_type":"city","required":true,"autocomplete":true},{"name":"date","var_type":"date","required":true,"autocomplete":false}]'),
('general', 'Yes/No Question', 'Will {{event}} happen by {{date}}?', '[{"name":"event","var_type":"text","required":true,"autocomplete":false},{"name":"date","var_type":"date","required":true,"autocomplete":false}]');
