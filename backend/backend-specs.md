# Backend API Reference

Base URL: `http://localhost:3000` (or configured host/port).

All request/response bodies that are JSON use `Content-Type: application/json` unless noted.
All JSON API methods accept optional `surface` (string). If omitted, backend uses `surface = "unknown"` for analytics.

---

## Authentication

Protected routes accept `Authorization: Bearer <token>` or `X-API-Key: <token>`.

### Prompt Keeper client API keys (`api_tokens`)

These are keys **we** issue (stored hashed in `api_tokens`). They are **not** the same as LLM provider secrets submitted via `POST /v1/keys` (stored in the `api_keys` vault table).

| Scope | Prefix (new keys) | Use |
|--------|-------------------|-----|
| **Management** (`mgt`) | `pk_mgt_live_` | CLI / automation: execute, store prompts and provider keys, mint execution keys. |
| **Execution** (`exe`) | `pk_exe_live_` | Embed in apps: `POST /v1/execute` and `GET /v1/list-prompts`. |

**Key format:** `pk_mgt_live_` or `pk_exe_live_` + 64 hexadecimal characters (32 bytes entropy) + `_` + 4 hexadecimal characters (checksum). Only these shapes are accepted for client API keys.

**Registration** returns a management key plus `api_key_scope: "mgt"`. **Mint** an execution key with `POST /v1/auth/api-tokens` using a management key or a session token from login.

**Restrictions:** An execution-only key receives **403 Forbidden** on mutating requests other than `POST /v1/execute` (e.g. `POST /v1/keys`, `POST /v1/prompts`, `POST /v1/auth/api-tokens`). It may call **`GET /v1/list-prompts`**. Session tokens and management keys are not subject to this restriction.

### Session tokens

`POST /v1/auth/login` returns a 64-character hex session token; use it like a client key for management operations.

---

## Endpoints

### 1. Execute (LLM proxy with streaming)

Runs the execute pipeline: resolves function config, renders the prompt with variables, forwards to the configured LLM provider (OpenAI, Anthropic, or Google Gemini), and streams the response back as Server-Sent Events.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/execute` |
| **Request body** | JSON (see parameters below) |
| **Response** | `text/event-stream` (SSE) |
| **Timeout** | 30s for the execute phase; stream may continue until provider closes |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `function_id` | string | Yes | Identifier of the function (e.g. `customer_support_reply`). Used to look up prompt template and provider config. |
| `variables` | object | No | Map of variable names to JSON values. Injected into the prompt template (Handlebars). Default: `{}`. |
| `provider` | string | No | Preferred provider (e.g. `"openai"`, `"anthropic"`). If in the function's provider list, tried first. |
| `model` | string | No | Model override. Takes precedence over prompt version default. If omitted everywhere, provider chooses (for Anthropic: `claude-sonnet-4-6`). |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Example request body:**

```json
{
  "function_id": "default",
  "variables": {
    "name": "Alice",
    "query": "What is the return policy?"
  },
  "provider": "anthropic",
  "model": "claude-3-5-sonnet-20240620"
}
```

#### Return value (success)

| Name | Type | Description |
|------|------|-------------|
| (stream) | SSE events | A stream of Server-Sent Events. Each event has a `data` field. Success: `data` contains provider payload (e.g. OpenAI/Anthropic/Gemini stream chunk). Events are sent until the provider closes the stream. |

#### Return value (error)

When the request is invalid or execute fails, the response is still SSE: a single event with JSON `data`:

| Name | Type | Description |
|------|------|-------------|
| `error` | string | Human-readable error message (e.g. parse failure, function not found, provider error, timeout). |

**Example error event (JSON in SSE `data`):**

```json
{
  "error": "function not found: unknown_fn"
}
```

**HTTP status:** On parse failure or timeout the stream still returns `200 OK` with an SSE stream whose first (and possibly only) event carries the `error` object. Provider or internal errors are also returned as SSE error events.

**Auth:** Management or execution client key, or session token. Execution-only keys are allowed for this endpoint.

---

### 1b. List prompts (titles only)

Returns distinct **function names** (prompt titles) that have a `production` deployment for the caller’s workspace **or** global scope (`context_id` `''`). Sorted by name. No template or provider details.

| Property | Value |
|----------|--------|
| **Method** | `GET` |
| **Path** | `/v1/list-prompts` |
| **Query** | `surface` (optional; default `"unknown"`) |
| **Response** | JSON |

#### Query parameters

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `surface` | string | No | Client surface for analytics. Default: `"unknown"`. |

#### Return value (success, 200)

| Name | Type | Description |
|------|------|-------------|
| `titles` | array of string | Prompt / function names only. |

**Auth:** Management or execution client key, or session token.

#### Return value (error)

JSON `{ "error": "..." }`. 401 (auth), 500 (database).

---

### 2a. Put key (store provider API key)

Stores a provider API key (e.g. OpenAI, Anthropic, Google Gemini). Uses envelope encryption (DEK + KMS). Raw secret is never logged. Requires KMS and auth.

**Auth:** Management client key or session token. Execution-only client keys receive **403**.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/keys` |
| **Request body** | JSON (see below) |
| **Response** | JSON, 201 Created |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `raw_secret` | string | Yes | Raw API key. Never logged. |
| `provider` | string | Yes | Provider (e.g. `"openai"`, `"anthropic"`, `"gemini"`). |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Example request body:**

```json
{
  "raw_secret": "sk-...",
  "provider": "openai"
}
```

#### Return value (success, 201)

`version_id`, `created_at`, `kms_key_arn`, `fingerprint`. `Location`: `/v1/keys`.

---

### 2b. Put prompt (store prompt template)

Stores a prompt template for a named function. Uses envelope encryption. Raw secret is never logged. Requires KMS and auth.

**Auth:** Management client key or session token. Execution-only client keys receive **403**.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/prompts` |
| **Request body** | JSON (see below) |
| **Response** | JSON, 201 Created |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `name` | string | Yes | Function/prompt name (e.g. `"customer_support"`). |
| `raw_secret` | string | Yes | Raw prompt template (e.g. Handlebars). Never logged. |
| `provider` | string | No | Optional default provider when creating a new function. |
| `preferred_model` | string | No | Default model for this version (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-20240620"`). Stored in `prompt_versions`; changes create a new version. |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Example request body:**

```json
{
  "name": "customer_support",
  "raw_secret": "Hello {{name}}!",
  "provider": "openai",
  "preferred_model": "gpt-4o"
}
```

#### Return value (success, 201)

`version_id`, `created_at`, `kms_key_arn`, `fingerprint`. `Location`: `/v1/functions/{name}/versions/{version_id}`.

#### Return value (error, both Put key and Put prompt)

JSON body: `{ "error": "<message>" }`. 400 (validation), 401 (auth), 403 (execution-only client key), 503 (no KMS), 502 (KMS failure), 500.

---

### 3. Register (create user)

Creates a new user with email, password (Argon2id), and optional name. Email is normalized to lowercase; password must be at least 12 characters. Requires `DATABASE_URL`.

**Proof-of-work:** Clients must send `X-Pow-Nonce`, `X-Pow-Solution`, and `X-Pow-Valid-Until` (see `GET /v1/auth/register-challenge` and the backend README).

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/register` |
| **Request body** | JSON (see below) |
| **Response** | JSON |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `email` | string | Yes | User email (must contain `@`). Normalized to lowercase. |
| `password` | string | Yes | Password; must be ≥ 12 characters. Stored only as Argon2id hash. |
| `name` | string | No | Display name. |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Example request body:**

```json
{
  "email": "user@example.com",
  "password": "securePassword123",
  "name": "Alice"
}
```

#### Return value (success, 201)

| Name | Type | Description |
|------|------|-------------|
| `id` | UUID | User ID. |
| `email` | string | Registered email. |
| `name` | string \| null | Display name, if provided. |
| `created_at` | string (ISO 8601) | Creation timestamp. |
| `default_workspace_id` | UUID | Default workspace created at signup. |
| `api_key` | string | **Management** client API key for the workspace (returned only once; store securely). Format: `pk_mgt_live_` + 64 hex + `_` + 4 hex checksum. |
| `api_key_scope` | string | Always `"mgt"` for this key. |

#### Return value (error)

JSON body: `{ "error": "<message>" }`.

| HTTP status | When |
|-------------|------|
| 400 Bad Request | Invalid email or password too short. |
| 409 Conflict | Email already registered. |
| 500 Internal Server Error | Hashing or DB failure. |

---

### 4. Mint execution API token (`POST /v1/auth/api-tokens`)

Creates an **execution-only** client API key (`pk_exe_live_...`). Callers may use a **management** client key or a **session** token (not an execution-only key).

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/api-tokens` |
| **Request body** | JSON (see below) |
| **Response** | JSON, 201 Created |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `label` | string | No | Label for the token. Default: `"Execution"`. |
| `surface` | string | No | Client-facing interface tag. Default: `"unknown"`. |

#### Return value (success, 201)

| Name | Type | Description |
|------|------|-------------|
| `api_key` | string | Execution-only key (returned only once; store securely). |
| `scope` | string | Always `"exe"`. |
| `label` | string | Stored label. |

#### Return value (error)

JSON body: `{ "error": "<message>" }`. 401 (auth), 403 (execution-only caller), 500.

---

### 5. Login (create session)

Verifies email and password, creates a session, and returns a session token. Uses generic "invalid email or password" on any auth failure to avoid user enumeration. Requires `DATABASE_URL`.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/login` |
| **Request body** | JSON (see below) |
| **Response** | JSON |

#### Request parameters (JSON body)

| Name | Type | Mandatory | Description |
|------|------|-----------|-------------|
| `email` | string | Yes | User email. |
| `password` | string | Yes | User password. |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Example request body:**

```json
{
  "email": "user@example.com",
  "password": "securePassword123"
}
```

#### Return value (success, 200)

| Name | Type | Description |
|------|------|-------------|
| `token` | string | Session token (hex, 64 chars). Send in `Authorization: Bearer <token>`. |
| `expires_at` | string (ISO 8601) | Session expiry (7 days from login). |
| `user` | object | `{ id, email, name }`. |

**Example response body:**

```json
{
  "token": "a1b2c3...",
  "expires_at": "2025-02-12T12:00:00Z",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "name": "Alice"
  }
}
```

#### Return value (error)

JSON body: `{ "error": "invalid email or password" }` or `{ "error": "login failed" }`.

| HTTP status | When |
|-------------|------|
| 401 Unauthorized | Invalid email format or credentials. |
| 500 Internal Server Error | DB or session creation failure. |

---

## Summary table

| Method | Path | Request body | Response | Mandatory params |
|--------|------|--------------|----------|------------------|
| GET | `/v1/list-prompts` | Query: `surface`? | JSON: `titles` | — |
| POST | `/v1/execute` | JSON: `function_id`, `variables`, `provider`? | SSE stream | `function_id` |
| POST | `/v1/keys` | JSON: `raw_secret`, `provider` | JSON | `raw_secret`, `provider` |
| POST | `/v1/prompts` | JSON: `name`, `raw_secret`, `provider`? | JSON | `name`, `raw_secret` |
| POST | `/v1/auth/register` | JSON: `email`, `password`, `name`? | JSON: user + `api_key`, `api_key_scope` | `email`, `password` |
| POST | `/v1/auth/api-tokens` | JSON: `label`?, `surface`? | JSON: `api_key`, `scope`, `label` | — (auth required) |
| POST | `/v1/auth/login` | JSON: `email`, `password` | JSON: `token`, `user` | `email`, `password` |
