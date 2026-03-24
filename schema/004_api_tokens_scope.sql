-- =============================================================================
-- api_tokens: scope for Prompt Keeper client keys (not LLM provider vault keys).
-- mgt = full RWX (CLI / management). exe = read+execute only (app embed keys).
-- =============================================================================

ALTER TABLE api_tokens
    ADD COLUMN IF NOT EXISTS scope TEXT NOT NULL DEFAULT 'mgt'
        CHECK (scope IN ('mgt', 'exe'));

COMMENT ON COLUMN api_tokens.scope IS 'mgt: management key (PUT prompts/keys, execute). exe: execute-only (POST /v1/execute).';
