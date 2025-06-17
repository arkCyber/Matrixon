-- Migration: Add typing notification tables
-- Author: arkSong <arksong2018@gmail.com>
-- Date: 2024-03-21
-- Version: 0.11.0-alpha

-- Create trusted_servers table
CREATE TABLE IF NOT EXISTS trusted_servers (
    id BIGSERIAL PRIMARY KEY,
    server_name TEXT NOT NULL UNIQUE,
    public_key TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on server_name for efficient lookups
CREATE INDEX IF NOT EXISTS idx_trusted_servers_name 
ON trusted_servers(server_name);

-- Create index on is_active for efficient filtering
CREATE INDEX IF NOT EXISTS idx_trusted_servers_active 
ON trusted_servers(is_active);

-- Add trigger for updated_at
CREATE TRIGGER update_trusted_servers_updated_at
    BEFORE UPDATE ON trusted_servers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Create typing events table
CREATE TABLE IF NOT EXISTS typing_events (
    id BIGSERIAL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    typing_data JSONB NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_id, user_id)
);

-- Create index on room_id and timestamp for efficient querying
CREATE INDEX IF NOT EXISTS idx_typing_events_room_timestamp 
ON typing_events(room_id, timestamp);

-- Create index on user_id for efficient user-based queries
CREATE INDEX IF NOT EXISTS idx_typing_events_user 
ON typing_events(user_id);

-- Create room_servers table for tracking which servers are in a room
CREATE TABLE IF NOT EXISTS room_servers (
    id BIGSERIAL PRIMARY KEY,
    room_id TEXT NOT NULL,
    server_name TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_id, server_name)
);

-- Create index on room_id for efficient room-based queries
CREATE INDEX IF NOT EXISTS idx_room_servers_room 
ON room_servers(room_id);

-- Create index on server_name for efficient server-based queries
CREATE INDEX IF NOT EXISTS idx_room_servers_server 
ON room_servers(server_name);

-- Add triggers for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_typing_events_updated_at
    BEFORE UPDATE ON typing_events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_room_servers_updated_at
    BEFORE UPDATE ON room_servers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add cleanup function for old typing events
CREATE OR REPLACE FUNCTION cleanup_old_typing_events()
RETURNS void AS $$
BEGIN
    DELETE FROM typing_events
    WHERE timestamp < NOW() - INTERVAL '1 hour';
END;
$$ language 'plpgsql';

-- Create a scheduled job to clean up old typing events
SELECT cron.schedule(
    'cleanup-typing-events',
    '0 * * * *',  -- Run every hour
    $$SELECT cleanup_old_typing_events()$$
); 
