CREATE TABLE users (
    id            TEXT(36) PRIMARY KEY,
    username      TEXT     NOT NULL UNIQUE,
    email         TEXT     NOT NULL UNIQUE,
    display_name  TEXT     NOT NULL,
    password_hash TEXT     NOT NULL,
    status        TEXT     NOT NULL DEFAULT 'active',
    created_at    TEXT     NOT NULL,
    updated_at    TEXT     NOT NULL
);

CREATE TABLE mcp_tokens (
    id           TEXT(36) PRIMARY KEY,
    user_id      TEXT(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_name   TEXT     NOT NULL,
    token_hash   TEXT     NOT NULL UNIQUE,
    scopes       TEXT     NOT NULL,
    status       TEXT     NOT NULL DEFAULT 'active',
    expires_at   TEXT,
    last_used_at TEXT,
    revoked_at   TEXT,
    created_at   TEXT     NOT NULL
);

CREATE INDEX idx_mcp_tokens_user_id ON mcp_tokens(user_id);
