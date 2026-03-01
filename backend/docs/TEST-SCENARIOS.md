# API Test Scenarios

Concise description of tested scenarios for each public API endpoint.

---

## 1. Health (`GET /health`)

| Scenario | Expected |
|----------|----------|
| Valid request | 200, body `"ok"` |
| Unknown route | 404 |
| POST (wrong method) | 405 Method Not Allowed |

---

## 2. Execute (`POST /v1/execute`)

| Scenario | Expected |
|----------|----------|
| Valid function_id + variables + provider | 200, `text/event-stream` |
| Unknown function_id | 200, SSE with `{"error": "function not found: <id>"}` |
| Invalid JSON body | 200, SSE with `{"error": "..."}` (parse error) |
| Missing function_id | 200, SSE error mentioning `function_id` |
| Empty function_id | 200, SSE `{"error": "function not found: "}` |
| Omitted variables | 200 (defaults to `{}`) |
| Wrong Content-Type (e.g. text/plain) | 200 (handler parses raw body) |
| Provider disabled, no fallback (test_fn_disabled) | 200, SSE error "disabled" or "not enabled" |
| Provider unsupported, no fallback (provider override) | 200, SSE error "not supported" |

---

## 3. Put key (`POST /v1/keys`) and Put prompt (`POST /v1/prompts`)

| Scenario | Expected |
|----------|----------|
| Without auth | 401 |
| KMS not configured | 503 |
| Key without provider | 400 or 503 |
| Prompt without name | 400 or 503 |
| Key: unknown fields | 422 |
| Prompt: missing raw_secret | 422 |
| Key: disabled provider (test_provider_disabled) | 400, error "not enabled" or "disabled" |
| Key: unsupported provider (test_provider_unsupported) | 400, error "not supported" |

---

## 4. Register (`POST /v1/auth/register`)

| Scenario | Expected |
|----------|----------|
| Valid email, password (≥12), name | 201, user + workspace + api_key; no password fields |
| Unknown fields (e.g. "admin") | 422 |
| Empty body / missing required fields | 422 |
| Empty email | 400, `{"error": "invalid email"}` |
| Invalid email format | 400, `{"error": "invalid email"}` |
| Password &lt; 12 chars | 400, `{"error": "password must be at least 12 characters"}` |
| Password exactly 11 chars | 400 |
| Email normalized to lowercase | 201, returned email is lowercase |
| Name omitted | 201, name is null/absent |
| Duplicate email | 409, `{"error": "email already registered"}` |

**Response checks:** `api_key` is `pk_` + 64 hex chars; no `password` or `password_hash` in response.

---

## 5. Login (`POST /v1/auth/login`)

| Scenario | Expected |
|----------|----------|
| Valid credentials (after register) | 200, token (64 hex), expires_at (ISO 8601), user object |
| Empty body / missing fields | 422 |
| Invalid email format | 401, `{"error": "invalid email or password"}` |
| Nonexistent user | 401, generic message (no enumeration) |
| Wrong password | 401, `{"error": "invalid email or password"}` |

**Response checks:** Token 64 hex chars; user object has no `password`; `expires_at` is ISO 8601.

---

## Dependencies

| Tests | Requires |
|-------|----------|
| Health, Execute, Put key, Put prompt, Register (validation), Login (validation) | None |
| Register (happy path, duplicate, normalize, missing name), Login (happy path, wrong password) | `DATABASE_URL`, schema applied |
| Put key (disabled/unsupported provider), Execute (provider disabled/unsupported) | `DATABASE_URL`, schema 001+004 (004 seeds test_provider_disabled) |
