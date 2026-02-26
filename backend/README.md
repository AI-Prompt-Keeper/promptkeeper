# Prompt Keeper — Rust backend

Self-contained Rust service: LLM proxy (execute), envelope encryption (Put), authentication, and static file fallback.

## Layout

- **`Cargo.toml`** — crate manifest and dependencies
- **`src/`** — application source
- **`backend-specs.md`** — detailed API reference (endpoints, request/response schemas)
- **`Dockerfile`** — image for local dev (build context from repo root)

## Build and run

From this directory:

```bash
cargo build --release
cargo run --release
```

Or from repo root:

```bash
cd backend && cargo run
```

To serve the project frontend (static site in `frontend/`):

```bash
cd backend && STATIC_DIR=../frontend cargo run
```

**Environment:** Requires `DATABASE_URL` for auth/registration. Optional `KMS_KEY_ID` (and AWS credentials) for envelope encryption endpoints.

**Schema:** Run `schema/001_prompt_management.sql`, `002_auth_and_workspaces.sql`, `003_api_tokens.sql` (in order). 001 includes functions, prompt_versions, deployments for Put/Execute.

See the repo root **DEPLOY.md** for full local deployment (DB, env vars, Docker).

## Tests

```bash
cargo test
```

Requires `DATABASE_URL` and the schema (users, workspaces, workspace_members, api_tokens). Tests will fail if the database is not set up.

See [docs/TEST-SCENARIOS.md](docs/TEST-SCENARIOS.md) for a concise list of tested scenarios per endpoint.

---

## API Reference

Base URL: `http://localhost:3000` (or configured host/port).

All JSON endpoints use `Content-Type: application/json` unless noted. Error responses are JSON: `{ "error": "<message>" }`.

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check for load balancers and Docker. |

**Response (200):** Plain text `ok`.

---

### 1. Execute — LLM proxy with streaming

Runs the execute pipeline: resolves function config, renders the prompt with variables (Handlebars), forwards to the configured LLM provider (OpenAI/Anthropic), and streams the response as Server-Sent Events.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/execute` |
| **Request** | JSON body |
| **Response** | `text/event-stream` (SSE) |
| **Timeout** | 30s for execute phase; stream continues until provider closes |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `function_id` | string | Yes | Function identifier (e.g. `default`, `customer_support_reply`). Used to look up prompt template and provider config. |

**Auth:** Requires `Authorization: Bearer <api_token>` or `X-API-Key: <api_token>`. Use the API key returned at registration (e.g. `pk_...`) or a session token from login.
| `variables` | object | No | Map of variable names to JSON values. Injected into the Handlebars prompt template. Default: `{}`. |
| `provider` | string | No | Preferred provider (e.g. `"openai"`, `"anthropic"`). If in the function's provider list, tried first. |

**Example request:**
```json
{
  "function_id": "default",
  "variables": {
    "name": "Alice",
    "query": "What is the return policy?"
  },
  "provider": "anthropic"
}
```

**Success response:** SSE stream. Each event has a `data` field containing provider payload (e.g. OpenAI/Anthropic stream chunks). Stream continues until the provider closes.

**Error response:** SSE stream with a single event whose `data` is JSON:
```json
{ "error": "function not found: unknown_fn" }
```

Common errors: parse failure, function not found, provider error, timeout (`"execute exceeded 30s client timeout"`). HTTP status remains 200; errors are delivered in SSE `data`.

---

### 2a. Put key — store provider API key

Stores a provider API key (e.g. OpenAI, Anthropic). Uses envelope encryption (DEK + KMS). Raw secret is never logged. Requires KMS and auth.

**Auth:** Requires `Authorization: Bearer <api_token>` or `X-API-Key: <api_token>`.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/keys` |
| **Request** | JSON body |
| **Response** | JSON, 201 Created |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `raw_secret` | string | Yes | Raw API key. Never logged; zeroized after use. |
| `provider` | string | Yes | Provider name (e.g. `"openai"`, `"anthropic"`). |

**Example:**
```json
{
  "raw_secret": "sk-...",
  "provider": "openai"
}
```

**Success response (201):** `version_id`, `created_at`, `kms_key_arn`, `fingerprint`. `Location`: `/v1/keys`.

---

### 2b. Put prompt — store prompt template

Stores a prompt template for a named function. Uses envelope encryption. Raw secret is never logged. Requires KMS and auth.

**Auth:** Requires `Authorization: Bearer <api_token>` or `X-API-Key: <api_token>`.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/prompts` |
| **Request** | JSON body |
| **Response** | JSON, 201 Created |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Function/prompt name (e.g. `"customer_support"`). |
| `raw_secret` | string | Yes | Raw prompt template (e.g. Handlebars). Never logged. |
| `provider` | string | No | Optional default provider (e.g. `"openai"`) when creating a new function. |

**Example:**
```json
{
  "name": "customer_support",
  "raw_secret": "Hello {{name}}! You asked: {{query}}",
  "provider": "openai"
}
```

**Success response (201):** `version_id`, `created_at`, `kms_key_arn`, `fingerprint`. `Location`: `/v1/functions/{name}/versions/{version_id}`.

**Error responses (both Put key and Put prompt):**

| Status | When |
|--------|------|
| 400 Bad Request | Missing required field or validation error. |
| 401 Unauthorized | Missing or invalid auth token. |
| 503 Service Unavailable | KMS not configured. |
| 502 Bad Gateway | KMS connection or config failed. |
| 500 Internal Server Error | Encryption or storage failed. |

---

### 3. Register — create user

Creates a new user with email and password. Also creates a default workspace, adds the user as owner, and issues an API key for that workspace. Email is normalized to lowercase; password must be at least 12 characters. Requires `DATABASE_URL`.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/register` |
| **Request** | JSON body |
| **Response** | JSON, 201 Created |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `email` | string | Yes | User email (must contain `@`). Normalized to lowercase. |
| `password` | string | Yes | Password; must be ≥ 12 characters. Stored as Argon2id hash only. |
| `name` | string | No | Display name. |

**Example request:**
```json
{
  "email": "user@example.com",
  "password": "securePassword123",
  "name": "Alice"
}
```

**Success response (201):**

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | User ID. |
| `email` | string | Registered email. |
| `name` | string \| null | Display name, if provided. |
| `created_at` | string (ISO 8601) | Creation timestamp. |
| `default_workspace_id` | UUID | Default workspace created at signup. |
| `api_key` | string | API key for the default workspace. **Returned only once**; store securely. Format: `pk_` + 64 hex chars. |

**Error responses:**

| Status | When |
|--------|------|
| 400 Bad Request | Invalid email or password too short. |
| 409 Conflict | Email already registered. |
| 500 Internal Server Error | Hashing, DB, or transaction failure. |

---

### 5. Login — create session

Verifies email and password, creates a session, and returns a session token. Uses generic "invalid email or password" on any auth failure to avoid user enumeration. Requires `DATABASE_URL`.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/login` |
| **Request** | JSON body |
| **Response** | JSON |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `email` | string | Yes | User email. |
| `password` | string | Yes | User password. |

**Example request:**
```json
{
  "email": "user@example.com",
  "password": "securePassword123"
}
```

**Success response (200):**

| Field | Type | Description |
|-------|------|-------------|
| `token` | string | Session token (64 hex chars). Send as `Authorization: Bearer <token>`. |
| `expires_at` | string (ISO 8601) | Session expiry (7 days from login). |
| `user` | object | `{ id, email, name }`. |

**Example response:**
```json
{
  "token": "a1b2c3d4e5f6...",
  "expires_at": "2025-02-12T12:00:00Z",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "name": "Alice"
  }
}
```

**Error responses:**

| Status | When |
|--------|------|
| 401 Unauthorized | Invalid email format or credentials. |
| 500 Internal Server Error | DB or session creation failure. |

---

## Summary

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | — | Health check |
| POST | `/v1/execute` | API key | Run function, stream LLM response |
| POST | `/v1/keys` | API key | Store provider API key (KMS required) |
| POST | `/v1/prompts` | API key | Store prompt template (KMS required) |
| POST | `/v1/auth/register` | — | Create user, workspace, and API key |
| POST | `/v1/auth/login` | — | Create session token |

**Note:** Execute and Put are gated by auth. Keys → `api_keys`; Prompts → `prompt_versions` + deployments.

For full request/response schemas and examples, see **backend-specs.md**.
