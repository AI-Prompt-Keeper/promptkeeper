-- =============================================================================
-- API Tokens: "Our" API keys (client auth) — one per (user_id, workspace_id)
-- Token is returned once at creation; only hash stored.
-- =============================================================================

CREATE TABLE api_tokens (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_id  UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    token_hash    TEXT NOT NULL,             -- SHA-256 hex of token; never store plaintext
    label         TEXT NOT NULL DEFAULT 'Default',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_api_tokens_hash ON api_tokens (token_hash);
CREATE INDEX idx_api_tokens_user_workspace ON api_tokens (user_id, workspace_id);
