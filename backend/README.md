# Prompt Keeper — Rust backend

Self-contained Rust service: LLM proxy (execute), envelope encryption (Put), authentication, and static file fallback.

## Layout

- **`Cargo.toml`** — crate manifest and dependencies
- **`src/`** — application source
- **`backend-specs.md`** — detailed API reference (endpoints, request/response schemas)
- **`Dockerfile`** — image for local dev (build context from repo root)
- **`Dockerfile.release`** — production image for Fly.io (multi-stage Rust build)

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

**Schema:** Run `schema/001_prompt_management.sql`, `002_auth_and_workspaces.sql`, `003_api_tokens.sql`, `004_api_tokens_scope.sql` (in order). 001 includes functions, prompt_versions, deployments for Put/Execute. `004` adds `scope` (`mgt` \| `exe`) to **client** API tokens (`api_tokens`).

See the repo root **DEPLOY.md** for full local deployment (DB, env vars, Docker).

## Deploy to Fly.io

From repo root: `fly deploy` (uses `fly.toml` and `backend/Dockerfile.release`). The backend **requires Postgres**; you can use Fly’s managed Postgres or an external database.

**First-time setup:**

1. **Create app:** `fly launch --no-deploy` then edit `fly.toml` as needed.
   - When asked “Do you want to tweak these settings?”, choose **Yes** if you want to add **Postgres** now (Fly will create a Postgres app and set `DATABASE_URL` for you). Choosing **No** is fine — you can add Postgres later (step 2).
2. **Postgres (if not added at launch):**
   - **Option A — Fly Postgres:** Create a cluster and attach it to your app (this sets `DATABASE_URL` automatically):
     ```bash
     fly postgres create
     fly postgres attach <postgres-app-name> --app promptkeeper
     ```
   - **Option B — External DB:** Set the secret yourself: `fly secrets set DATABASE_URL="postgres://user:pass@host:5432/dbname"`.
   - After attaching Fly Postgres or using an external DB, run the schema migrations (e.g. from a one-off machine or locally against the same URL): `schema/001_prompt_management.sql`, `002_auth_and_workspaces.sql`, `003_api_tokens.sql`, `004_api_tokens_scope.sql`.
3. **Other secrets (optional):** `fly secrets set KMS_KEY_ID=... AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... AWS_REGION=...` for envelope encryption.
4. **CI:** Pushes to `main` deploy via GitHub Actions (`.github/workflows/fly-deploy.yml`). Add repository secret `FLY_API_TOKEN` from `fly tokens create deploy -x 999999h`.

## Tests

```bash
cargo test
```

Requires `DATABASE_URL` and the schema (users, workspaces, workspace_members, `api_tokens` including `scope`). Tests will fail if the database is not set up.

See [docs/TEST-SCENARIOS.md](docs/TEST-SCENARIOS.md) for a concise list of tested scenarios per endpoint.

---

## API Reference

Base URL: `http://localhost:3000` (or configured host/port).

All JSON endpoints use `Content-Type: application/json` unless noted. Error responses are JSON: `{ "error": "<message>" }`.
All JSON API methods accept optional `surface` (string). If omitted, backend uses `surface = "unknown"` for analytics.

### Client API keys (Prompt Keeper tokens)

Keys **we** issue to customers are stored in **`api_tokens`** (hashed at rest). This is separate from **LLM provider** secrets stored via `POST /v1/keys`, which live in the vault table **`api_keys`**.

| Scope | Prefix | Access |
|--------|--------|--------|
| **Management** | `pk_mgt_live_` | Full: `POST /v1/execute`, `POST /v1/keys`, `POST /v1/prompts`, `POST /v1/auth/api-tokens`. |
| **Execution** | `pk_exe_live_` | `POST /v1/execute` and **`GET /v1/list-prompts`**. Other routes → **403** (including mutating routes other than execute). |

**Format:** `pk_mgt_live_` or `pk_exe_live_` + 64 hex characters (32 bytes of entropy) + `_` + 4 hex characters (checksum). Total length **81** characters. Other shapes are **rejected**.

Registration returns a **management** key (`api_key`) and `api_key_scope: "mgt"`. To create an **execution** key, call `POST /v1/auth/api-tokens` with a management API key or a **session** token from login (not with an execution-only key).

**Auth header:** `Authorization: Bearer <token>` or `X-API-Key: <token>`. Never log full tokens in application logs.

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check for load balancers and Docker. |

**Response (200):** Plain text `ok`.

---

### 1. Execute — LLM proxy with streaming

Runs the execute pipeline: resolves function config, renders the prompt with variables (Handlebars), forwards to the configured LLM provider (OpenAI, Anthropic, or Google Gemini), and streams the response as Server-Sent Events.

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
| `variables` | object | No | Map of variable names to JSON values. Injected into the Handlebars prompt template. Default: `{}`. |
| `provider` | string | No | Preferred provider (e.g. `"openai"`, `"anthropic"`, `"gemini"`). If in the function's provider list, tried first. |
| `model` | string | No | Model override. If omitted, configured/provider defaults apply (Anthropic defaults to `claude-sonnet-4-6`). |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

**Auth:** Requires `Authorization: Bearer <token>` or `X-API-Key: <token>`. Accepts a **management** client key, an **execution** client key, or a **session** token from login.

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

**Success response:** SSE stream. Each event has a `data` field containing provider payload (e.g. OpenAI/Anthropic/Gemini stream chunks). Stream continues until the provider closes.

**Error response:** SSE stream with a single event whose `data` is JSON:
```json
{ "error": "function not found: unknown_fn" }
```

Common errors: parse failure, function not found, provider error, timeout (`"execute exceeded 30s client timeout"`). HTTP status remains 200; errors are delivered in SSE `data`.

---

### 1b. List prompts — titles only

Returns the sorted list of **function names** (prompt titles) that have a **production** deployment for your workspace **or** global scope (`context_id` empty). No template text or metadata.

| Property | Value |
|----------|--------|
| **Method** | `GET` |
| **Path** | `/v1/list-prompts` |
| **Query** | Optional `surface` (default `"unknown"`). |
| **Response** | JSON `200` |

**Auth:** **Management** or **execution** client key, or **session** token.

**Success response (200):**

```json
{ "titles": ["default", "my_prompt"] }
```

**Error responses:** `401` (missing/invalid auth), `500` (database failure).

---

### 2a. Put key — store provider API key

Stores a provider API key (e.g. OpenAI, Anthropic, Google Gemini). Uses envelope encryption (DEK + KMS). Raw secret is never logged. Requires KMS and auth.

**Auth:** Requires a **management** client API key or a **session** token. **Execution-only** keys (`pk_exe_live_...`) receive **403 Forbidden**.

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
| `provider` | string | Yes | Provider name (e.g. `"openai"`, `"anthropic"`, `"gemini"`). |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

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

**Auth:** Requires a **management** client API key or a **session** token. **Execution-only** keys receive **403 Forbidden**.

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
| `provider` | string | No | Optional default provider (e.g. `"openai"`, `"gemini"`) when creating a new function. |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

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

**Proof-of-work:** Registration requires a valid PoW. First call `GET /v1/auth/register-challenge` to get `nonce`, `difficulty`, and `valid_until`. Find a `solution` such that `SHA256(nonce_bytes || valid_until || solution)` has ≥ `difficulty` leading zero bits. Then send `POST /v1/auth/register` with the same JSON body plus headers: `X-Pow-Nonce`, `X-Pow-Solution`, `X-Pow-Valid-Until`. See `cli/docs/REGISTER_POW.md` for client implementation details.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/register` |
| **Request** | JSON body + headers (see above) |
| **Response** | JSON, 201 Created |

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `email` | string | Yes | User email (must contain `@`). Normalized to lowercase. |
| `password` | string | Yes | Password; must be ≥ 12 characters. Stored as Argon2id hash only. |
| `name` | string | No | Display name. |
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

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
| `api_key` | string | **Management** API key for the default workspace. **Returned only once**; store securely. Format: `pk_mgt_live_` + 64 hex + `_` + 4 hex checksum (see **Client API keys** above). |
| `api_key_scope` | string | Always `"mgt"` for this key. |

**Error responses:**

| Status | When |
|--------|------|
| 400 Bad Request | Missing or invalid proof-of-work; invalid email or password too short; challenge expired. |
| 409 Conflict | Email already registered. |
| 500 Internal Server Error | Hashing, DB, or transaction failure. |

---

### 4. Mint execution API key

Creates an additional **execution-only** client API key (`pk_exe_live_...`, scope `exe`). Use in application code; it cannot create prompts, store provider keys, or mint further tokens.

| Property | Value |
|----------|--------|
| **Method** | `POST` |
| **Path** | `/v1/auth/api-tokens` |
| **Request** | JSON body |
| **Response** | JSON, 201 Created |

**Auth:** **Management** client API key or **session** token. Execution-only keys receive **403**.

**Request body (JSON):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `label` | string | No | Label for this token (e.g. `"iOS app"`). Default: `"Execution"`. |
| `surface` | string | No | Client-facing interface tag. Default: `"unknown"`. |

**Success response (201):**

| Field | Type | Description |
|-------|------|-------------|
| `api_key` | string | Execution-only key. **Returned only once**; store securely. |
| `scope` | string | Always `"exe"`. |
| `label` | string | Label stored with the token. |

**Error responses:** 401 (missing/invalid auth), 403 (caller used an execution-only key), 500.

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
| `surface` | string | No | Client-facing interface tag (e.g. `"cli"`, `"android"`, `"web"`). Default: `"unknown"`. |

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
| GET | `/v1/list-prompts` | Client key or session | List stored prompt **titles** (production deployments) |
| POST | `/v1/execute` | Client key or session | Run stored prompt by `function_id`, stream LLM response |
| POST | `/v1/keys` | Management key or session | Store **provider** API key in vault (KMS required) |
| POST | `/v1/prompts` | Management key or session | Store prompt template (KMS required) |
| POST | `/v1/auth/register` | — | Create user, workspace, and **management** client API key |
| POST | `/v1/auth/api-tokens` | Management key or session | Mint **execution-only** client API key |
| POST | `/v1/auth/login` | — | Create session token |

**Note:** Execute and Put are gated by auth. Execute requires a stored prompt (`POST /v1/prompts`) and a stored **provider** key (`POST /v1/keys`). Provider secrets → table **`api_keys`**; Prompts → `prompt_versions` + deployments. **Client** keys (management / execution) → **`api_tokens`**.

For full request/response schemas and examples, see **backend-specs.md**.
