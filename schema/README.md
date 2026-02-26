# Schema: AI Prompt Management & Auth

## Files (run in order)

| File | Description |
|------|-------------|
| `001_prompt_management.sql` | functions, prompt_versions, deployments (incl. context_id, envelope encryption) |
| `002_auth_and_workspaces.sql` | users, sessions (token_hash), workspaces, api_keys (envelope), workspace_members, mfa_settings, onboarding_state |
| `003_api_tokens.sql` | api_tokens (client auth; token_hash) |

## Prompt Management (001)

- **functions** — Top-level names (e.g. `default`, `customer_support_reply`) + provider routing (primary_provider, backup_providers, provider_config).
- **prompt_versions** — Immutable rows: `template_text` (plaintext) or envelope columns (encrypted_payload, encrypted_dek, nonce, kms_key_id), plus model_config, provider_settings, context_id.
- **deployments** — Maps (function_id, context_id, tag) → version_id. One active version per env per workspace (context_id = '' for global).

## Immutability

A trigger on `prompt_versions` blocks `UPDATE` and `DELETE`. To change a prompt, insert a new version and point the deployment to it.

## Performance: &lt;5ms active production lookup

**Index:** `UNIQUE (function_id, context_id, tag)` on `deployments`.

**Query (by function name + context):**

```sql
SELECT pv.*, f.primary_provider, f.backup_providers, f.provider_config
FROM deployments d
JOIN functions f ON f.id = d.function_id
JOIN prompt_versions pv ON pv.id = d.version_id
WHERE f.name = $1 AND d.tag = 'production' AND d.context_id = $2;
```

## Hot-swapping

1. Insert new row into `prompt_versions`.
2. `UPDATE deployments SET version_id = $new_id WHERE function_id = $f AND context_id = $ctx AND tag = 'production';`

## Security notes

- **sessions**: Store `token_hash` (SHA-256 hex) only; validate by hash. Backend must hash on insert.
- **api_keys**: Envelope encryption (key_encrypted, encrypted_dek, nonce, kms_key_id).
- **api_tokens**: Store token_hash only; plaintext never persisted.
