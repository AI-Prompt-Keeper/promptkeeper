-- Test-only: provider in catalog but disabled (enabled=false).
-- Used by integration tests for put_key and execute provider validation.
INSERT INTO supported_providers (provider, supported, enabled) VALUES
    ('test_provider_disabled', true, false)
ON CONFLICT (provider) DO UPDATE SET supported = true, enabled = false;
