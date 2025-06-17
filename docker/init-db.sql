-- =============================================================================
-- Matrixon PostgreSQL Database Initialization
-- =============================================================================
--
-- Project: Matrixon - Ultra High Performance Matrix NextServer
-- Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
-- Date: 2024-12-11
-- Version: 0.11.0-alpha
-- License: Apache 2.0 / MIT
--
-- Description:
--   PostgreSQL database initialization for Matrixon Matrix server
--   Performance optimized with proper indexes and constraints
--
-- =============================================================================

-- Ensure UTF8 encoding
SET client_encoding = 'UTF8';

-- Create extensions for better performance
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- Grant all necessary permissions to matrixon user
GRANT ALL PRIVILEGES ON DATABASE matrixon TO matrixon;
GRANT CREATE ON SCHEMA public TO matrixon;
GRANT USAGE ON SCHEMA public TO matrixon;

-- Create basic tables structure for Matrix protocol
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    display_name TEXT,
    avatar_url TEXT,
    is_admin BOOLEAN DEFAULT FALSE,
    is_guest BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Rooms table
CREATE TABLE IF NOT EXISTS rooms (
    id SERIAL PRIMARY KEY,
    room_id TEXT UNIQUE NOT NULL,
    room_version TEXT DEFAULT '10',
    creator TEXT NOT NULL,
    name TEXT,
    topic TEXT,
    avatar_url TEXT,
    join_rule TEXT DEFAULT 'invite',
    history_visibility TEXT DEFAULT 'shared',
    guest_access TEXT DEFAULT 'forbidden',
    is_public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Events table for Matrix events
CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    event_id TEXT UNIQUE NOT NULL,
    room_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    event_type TEXT NOT NULL,
    content JSONB,
    state_key TEXT,
    prev_events JSONB,
    auth_events JSONB,
    depth BIGINT,
    origin_server_ts BIGINT,
    unsigned_data JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (room_id) REFERENCES rooms(room_id)
);

-- Room memberships
CREATE TABLE IF NOT EXISTS room_memberships (
    id SERIAL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    membership TEXT NOT NULL DEFAULT 'join',
    event_id TEXT,
    joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(room_id, user_id),
    FOREIGN KEY (room_id) REFERENCES rooms(room_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

-- Access tokens
CREATE TABLE IF NOT EXISTS access_tokens (
    id SERIAL PRIMARY KEY,
    token TEXT UNIQUE NOT NULL,
    user_id TEXT NOT NULL,
    device_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

-- Create performance indexes
CREATE INDEX IF NOT EXISTS idx_users_user_id ON users(user_id);
CREATE INDEX IF NOT EXISTS idx_rooms_room_id ON rooms(room_id);
CREATE INDEX IF NOT EXISTS idx_events_room_id ON events(room_id);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_origin_server_ts ON events(origin_server_ts);
CREATE INDEX IF NOT EXISTS idx_room_memberships_room_id ON room_memberships(room_id);
CREATE INDEX IF NOT EXISTS idx_room_memberships_user_id ON room_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_access_tokens_token ON access_tokens(token);
CREATE INDEX IF NOT EXISTS idx_access_tokens_user_id ON access_tokens(user_id);

-- Create update trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_rooms_updated_at BEFORE UPDATE ON rooms
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Grant all permissions on tables to matrixon user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO matrixon;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO matrixon;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO matrixon;

-- Configure PostgreSQL for performance
ALTER SYSTEM SET shared_buffers = '512MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET maintenance_work_mem = '128MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
ALTER SYSTEM SET default_statistics_target = 100;
ALTER SYSTEM SET random_page_cost = 1.1;
ALTER SYSTEM SET effective_io_concurrency = 200;

-- Select configuration to apply settings
SELECT pg_reload_conf(); 
