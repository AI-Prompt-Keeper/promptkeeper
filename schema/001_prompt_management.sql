-- =============================================================================
-- AI Prompt Management Schema
-- Supports: functions, immutable prompt versions, deployments (production/staging/dev)
-- Workspace-scoped via context_id. Target: <5ms for active production version lookup.
-- Consolidates 001 + 004 (no migrations; single canonical schema).
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. FUNCTIONS (top-level function names + provider routing)
-- -----------------------------------------------------------------------------
CREATE TABLE functions (
    id                 BIGSERIAL PRIMARY KEY,
    name               TEXT NOT NULL UNIQUE,
    primary_provider   TEXT NOT NULL DEFAULT 'openai',
    backup_providers   JSONB NOT NULL DEFAULT '[]',
    response_format    TEXT,
    provider_config    JSONB NOT NULL DEFAULT '{}',
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE functions IS 'Top-level function identifiers (e.g. customer_support_reply, default)';
COMMENT ON COLUMN functions.primary_provider IS 'Primary LLM provider (e.g. openai, anthropic)';
COMMENT ON COLUMN functions.backup_providers IS 'Backup provider ids for failover';
COMMENT ON COLUMN functions.provider_config IS 'Per-provider config: url, model, etc.';

-- -----------------------------------------------------------------------------
-- 2. PROMPT_VERSIONS (immutable; template or encrypted + model config)
-- -----------------------------------------------------------------------------
CREATE TABLE prompt_versions (
    id                BIGSERIAL PRIMARY KEY,
    function_id       BIGINT NOT NULL REFERENCES functions(id) ON DELETE CASCADE,
    template_text     TEXT,                    -- plaintext; NULL when using envelope encryption
    model_config      JSONB NOT NULL DEFAULT '{}',
    provider_settings JSONB NOT NULL DEFAULT '{}',
    -- Envelope encryption (KMS + DEK)
    encrypted_payload BYTEA,
    encrypted_dek     BYTEA,
    nonce             BYTEA,
    kms_key_id        TEXT,
    context_id        TEXT NOT NULL DEFAULT '',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version_label     TEXT,
    CONSTRAINT prompt_versions_content_check CHECK (
        (template_text IS NOT NULL AND template_text != '') OR
        (encrypted_payload IS NOT NULL AND encrypted_dek IS NOT NULL AND nonce IS NOT NULL AND kms_key_id IS NOT NULL)
    )
);

COMMENT ON TABLE prompt_versions IS 'Immutable prompt versions; edit = create new version';
COMMENT ON COLUMN prompt_versions.model_config IS 'e.g. temperature, max_tokens';
COMMENT ON COLUMN prompt_versions.provider_settings IS 'Provider-specific overrides';
COMMENT ON COLUMN prompt_versions.encrypted_payload IS 'AES-GCM ciphertext when using envelope encryption';
COMMENT ON COLUMN prompt_versions.encrypted_dek IS 'KMS-wrapped DEK';
COMMENT ON COLUMN prompt_versions.context_id IS 'Workspace/scope for AAD; must match on decrypt';

CREATE INDEX idx_prompt_versions_function_created
    ON prompt_versions (function_id, created_at DESC);
CREATE INDEX idx_prompt_versions_function_context
    ON prompt_versions (function_id, context_id);

-- -----------------------------------------------------------------------------
-- 3. DEPLOYMENTS (function_id + context_id + tag -> version_id; one per env)
-- -----------------------------------------------------------------------------
CREATE TABLE deployments (
    id          BIGSERIAL PRIMARY KEY,
    function_id BIGINT NOT NULL REFERENCES functions(id) ON DELETE CASCADE,
    version_id  BIGINT NOT NULL REFERENCES prompt_versions(id) ON DELETE RESTRICT,
    tag         TEXT NOT NULL CHECK (tag IN ('production', 'staging', 'dev')),
    context_id  TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (function_id, context_id, tag)
);

COMMENT ON TABLE deployments IS 'Maps (function, context, tag) to the active prompt_version';
COMMENT ON COLUMN deployments.context_id IS 'Workspace/scope; empty string for global default';

-- UNIQUE (function_id, context_id, tag) above provides index for <5ms lookup

-- -----------------------------------------------------------------------------
-- IMMUTABILITY: Prevent UPDATE/DELETE on prompt_versions (only INSERT allowed)
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION enforce_prompt_version_immutable()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'prompt_versions are immutable: cannot UPDATE row id=%', OLD.id;
    END IF;
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'prompt_versions are immutable: cannot DELETE row id=%', OLD.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Use EXECUTE PROCEDURE instead of EXECUTE FUNCTION if on PostgreSQL 10 or older
CREATE TRIGGER trigger_prompt_versions_immutable
    BEFORE UPDATE OR DELETE ON prompt_versions
    FOR EACH ROW
    EXECUTE FUNCTION enforce_prompt_version_immutable();

-- -----------------------------------------------------------------------------
-- EXAMPLE QUERY: Fetch active production version (optimized)
-- -----------------------------------------------------------------------------
-- SELECT pv.*, f.name, f.primary_provider, f.backup_providers, f.provider_config
-- FROM deployments d
-- JOIN functions f ON f.id = d.function_id
-- JOIN prompt_versions pv ON pv.id = d.version_id
-- WHERE f.name = $1 AND d.tag = 'production' AND d.context_id = $2;
