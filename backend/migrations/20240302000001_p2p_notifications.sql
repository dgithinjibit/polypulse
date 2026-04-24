-- Add notification types for P2P betting events
-- This migration extends the existing notifications table with P2P bet-specific functionality

-- Add index for notification cleanup (30-day retention)
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);

-- Add index for notification type filtering
CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type);

-- Add bet_id column to notifications for P2P bet events
ALTER TABLE notifications ADD COLUMN IF NOT EXISTS bet_id BIGINT REFERENCES p2p_bets(id) ON DELETE CASCADE;

-- Add index for bet-related notifications
CREATE INDEX IF NOT EXISTS idx_notifications_bet ON notifications(bet_id);
