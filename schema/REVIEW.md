# Schema Review — Integration Readiness & Security

## Summary

| Schema | Used by Backend | Integration Ready | Notes |
|--------|-----------------|-------------------|-------|
| 001 (prompt_management) | Yes | Yes | Consolidated with former 004; DbFunctionStore loads from DB |
| 002 (auth_and_workspaces) | Partially | Partially | users, sessions, workspaces, workspace_members ✓; api_keys has envelope cols ✓; **sessions: backend must use token_hash** |
| 003 (api_tokens) | Yes | Yes | Matches register flow |

---

## 1. What's Used Today

**Backend currently uses:**
- `users` — register, login
- `sessions` — login (insert, lookup by token) — **schema now has token_hash; backend must be updated**
- `workspaces` — register (create default)
- `workspace_members` — register (add user as owner)
- `api_tokens` — register (create default API key)

**Used (schema 001):**
- `functions`, `prompt_versions`, `deployments` — Put persists prompts; Execute loads from DB cache

**Used:**
- `api_keys` — Put stores provider keys (openai, anthropic) per (user_id, workspace_id, provider)
- `mfa_settings`, `onboarding_state` — reserved for future

---

## 2. Applied Schema Changes (REVIEW feedback)

### 2.1. `api_keys` — envelope columns ✓
Added `encrypted_dek BYTEA`, `kms_key_id TEXT` for KMS envelope decryption.

### 2.2. `functions` / `prompt_versions` / `deployments` ✓
- `functions`: Added primary_provider, backup_providers, response_format, provider_config
- `prompt_versions`: Added encrypted_payload, encrypted_dek, nonce, kms_key_id, context_id; template_text nullable
- `deployments`: Added context_id; UNIQUE (function_id, context_id, tag)
- 004 merged into 001; 004 deleted

### 2.3. `sessions` — token_hash ✓ (BACKEND UPDATE REQUIRED)
Schema now has `token_hash` instead of `token`. **Backend must:**
1. On login: hash token (SHA-256 hex), store hash in sessions
2. On validate: hash incoming Bearer token, `SELECT ... WHERE token_hash = $1`

**Breaking:** Existing sessions will be invalid. Requires re-login.

---

## 3. Remaining Items

### 3.1. Session cleanup
Add periodic cleanup: `DELETE FROM sessions WHERE expires_at < now()` (cron or on login).

### 3.2. No Row-Level Security (RLS)
All authorization is in the app. For multi-tenant hardening, add RLS policies. Not strictly required if the app is the sole DB client.

### 3.3. `001` trigger syntax
Uses `EXECUTE FUNCTION` (PG 11+). Docker uses `postgres:16-alpine` — OK.

---

## 4. Integration Readiness Checklist

- [x] users, workspaces, workspace_members, api_tokens — used and match
- [x] sessions — **schema updated to token_hash** — [ ] backend must hash on insert/validate
- [x] api_keys — envelope columns (encrypted_dek, kms_key_id) added
- [x] functions, prompt_versions, deployments — consolidated in 001
- [ ] **Backend: update login to store token_hash, auth middleware to validate by hash**
