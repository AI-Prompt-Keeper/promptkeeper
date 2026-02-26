-- =============================================================================
-- Auth & Workspaces Schema
-- Users, sessions, MFA, encrypted API keys, workspaces
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. USERS (email/password or OAuth)
-- -----------------------------------------------------------------------------
CREATE TABLE users (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email          TEXT NOT NULL,
    password_hash  TEXT,                    -- NULL for OAuth-only users
    oauth_provider TEXT,                    -- 'github' | 'google' | NULL
    oauth_id       TEXT,                    -- provider's user id
    name           TEXT,
    email_verified_at TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (email),
    UNIQUE (oauth_provider, oauth_id)
);

CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_oauth ON users (oauth_provider, oauth_id);

-- -----------------------------------------------------------------------------
-- 2. SESSIONS (token_hash only; plaintext never stored — validate by hash)
-- -----------------------------------------------------------------------------
CREATE TABLE sessions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,          -- SHA-256 hex of token; never store plaintext
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions (token_hash);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- NOTE: Backend must hash token on login (e.g. SHA-256 hex) and store hash.
-- Validate: hash incoming Bearer token, SELECT ... WHERE token_hash = $1.
-- Periodic cleanup: DELETE FROM sessions WHERE expires_at < now() (cron or on login).

-- -----------------------------------------------------------------------------
-- 3. MFA SETTINGS (TOTP secret + backup codes; encrypted at rest)
-- -----------------------------------------------------------------------------
CREATE TABLE mfa_settings (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    enabled              BOOLEAN NOT NULL DEFAULT false,
    totp_secret_encrypted TEXT,             -- AES-256-GCM encrypted
    totp_secret_nonce     BYTEA,
    backup_codes_encrypted TEXT,            -- AES-256-GCM encrypted JSON array
    backup_codes_nonce     BYTEA,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- -----------------------------------------------------------------------------
-- 4. WORKSPACES (Personal, Team, Production, etc.)
-- -----------------------------------------------------------------------------
CREATE TABLE workspaces (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_workspaces_slug ON workspaces (slug);

-- -----------------------------------------------------------------------------
-- 5. API KEYS (envelope encryption: KMS + DEK; we never store plaintext)
-- -----------------------------------------------------------------------------
CREATE TABLE api_keys (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_id     UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    label            TEXT NOT NULL,             -- e.g. "OpenAI Production"
    key_encrypted    BYTEA NOT NULL,            -- AES-GCM ciphertext (encrypted_payload)
    nonce            BYTEA NOT NULL,            -- 12 bytes for GCM
    encrypted_dek    BYTEA,                     -- KMS-wrapped DEK (envelope encryption)
    kms_key_id       TEXT,                      -- KMS key ID/ARN used to wrap DEK
    provider         TEXT NOT NULL,             -- 'openai' | 'anthropic' | etc.
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_api_keys_user_workspace ON api_keys (user_id, workspace_id);

-- NOTE: For KMS envelope decryption: key_encrypted + encrypted_dek + nonce + kms_key_id.
-- Older rows may have key_encrypted + nonce only (non-envelope); encrypted_dek/kms_key_id NULL.

-- -----------------------------------------------------------------------------
-- 6. WORKSPACE MEMBERS (workspaces created above)
-- -----------------------------------------------------------------------------
CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role         TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX idx_workspace_members_user ON workspace_members (user_id);

-- -----------------------------------------------------------------------------
-- 7. ONBOARDING STATE (track if user completed key handshake / workspace)
-- -----------------------------------------------------------------------------
CREATE TABLE onboarding_state (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    api_key_added_at     TIMESTAMPTZ,
    default_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    completed_at         TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
