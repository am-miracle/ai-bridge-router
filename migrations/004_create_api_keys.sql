-- Create api_keys table for API key management
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    permissions JSONB NOT NULL DEFAULT '[]',
    rate_limit_per_minute INTEGER NOT NULL DEFAULT 100,
    rate_limit_per_hour INTEGER NOT NULL DEFAULT 1000,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    last_used_at TIMESTAMP,
    expires_at TIMESTAMP
);

-- Create an index on key_hash for fast lookups
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

-- Create an index on is_active for filtering active keys
CREATE INDEX idx_api_keys_is_active ON api_keys(is_active);

-- Create an index on created_at for sorting
CREATE INDEX idx_api_keys_created_at ON api_keys(created_at);

-- Insert a default admin API key (for initial setup)
-- Note: This is a placeholder key that should be changed immediately
INSERT INTO api_keys (
    id,
    key_hash,
    name,
    description,
    permissions,
    rate_limit_per_minute,
    rate_limit_per_hour,
    is_active
) VALUES (
    '00000000-0000-0000-0000-000000000001',
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', -- hash of 'admin_initial_key_change_me'
    'Admin Initial Key',
    'Default admin key - CHANGE IMMEDIATELY',
    '["admin:manage", "security:read"]',
    1000,
    10000,
    true
);
