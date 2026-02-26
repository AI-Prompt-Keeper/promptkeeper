# Local deployment: Backend, DB, and frontend

Run the full stack on your machine for testing. The backend serves both the API and the static frontend.

---

## 1. Prerequisites

- **Rust** (e.g. `rustup`)
- **PostgreSQL** 14+ (for schema; backend currently uses mock data until DB is wired)
- **OpenAI API key** (for `/v1/execute`; optional for other endpoints)
- **AWS credentials + KMS key** (optional; for `POST /v1/keys` and `POST /v1/prompts`)

---

## 2. Database (PostgreSQL)

Start PostgreSQL (e.g. local install or Docker):

```bash
# Example: Docker
docker run -d --name promptkeeper-db \
  -e POSTGRES_USER=promptkeeper \
  -e POSTGRES_PASSWORD=promptkeeper \
  -e POSTGRES_DB=promptkeeper \
  -p 5432:5432 \
  postgres:16-alpine
```

Create the schema (run once):

```bash
export PGHOST=localhost PGPORT=5432 PGPASSWORD=promptkeeper PGUSER=promptkeeper PGDATABASE=promptkeeper

psql -f schema/001_prompt_management.sql
psql -f schema/002_auth_and_workspaces.sql
psql -f schema/003_api_tokens.sql
```

The backend requires the schema for auth (register, login), Put, and Execute (execute and put use in-memory/mock data). Applying the schema prepares the database for when you wire it in.

---

## 3. Environment variables

Create a `.env` (or export in the shell):

| Variable | Required | Description |
|----------|----------|-------------|
| `OPENAI_API_KEY` | For execute | OpenAI API key; used by `/v1/execute` when provider is OpenAI. |
| `KMS_KEY_ID` | For Put | AWS KMS key ID or alias (e.g. `alias/my-key`). Enables `POST /v1/keys` and `POST /v1/prompts`. |
| `AWS_REGION` | If using KMS | e.g. `us-east-1`. |
| `STATIC_DIR` | No | Directory for static files. When running from `backend/`, set to `../frontend` to serve the project frontend (default in `backend/`: `.`). |

Minimal for execute only:

```bash
export OPENAI_API_KEY=sk-...
```

With Put:

```bash
export OPENAI_API_KEY=sk-...
export KMS_KEY_ID=alias/my-key
export AWS_REGION=us-east-1
# AWS credentials: env vars, ~/.aws/credentials, or IAM role
```

---

## 4. Build and run the backend

All Rust backend code lives in **`backend/`**. From the project root:

```bash
cd backend
cargo build --release
cargo run --release
```

Or for development:

```bash
cd backend
cargo run
```

To serve the project frontend (static site in `frontend/`), run from repo root:

```bash
cd backend && STATIC_DIR=../frontend cargo run
```

The server listens on **http://0.0.0.0:3000**.

- **API**: `http://localhost:3000/v1/execute`, `http://localhost:3000/v1/keys`, `http://localhost:3000/v1/prompts`
- **Frontend**: `http://localhost:3000/` (serves from `STATIC_DIR`; use `STATIC_DIR=../frontend` when in `backend/` to serve the `frontend/` directory)

---

## 5. Quick test

1. **Frontend**: Open [http://localhost:3000](http://localhost:3000) in a browser.
2. **Execute** (streaming):
   ```bash
   curl -N -X POST http://localhost:3000/v1/execute \
     -H "Content-Type: application/json" \
     -d '{"function_id":"default","variables":{"name":"Alice","query":"Hi"}}'
   ```
3. **Put prompt** (needs KMS + auth):
   ```bash
   curl -X POST http://localhost:3000/v1/prompts \
     -H "Authorization: Bearer pk_..." \
     -H "Content-Type: application/json" \
     -d '{"name":"my_func","raw_secret":"Hello {{name}}","provider":"openai"}'
   ```
4. **Put key** (needs KMS + auth):
   ```bash
   curl -X POST http://localhost:3000/v1/keys \
     -H "Authorization: Bearer pk_..." \
     -H "Content-Type: application/json" \
     -d '{"raw_secret":"sk-...","provider":"openai"}'
   ```

---

## Summary

| Component | How to run |
|-----------|------------|
| **DB** | Start PostgreSQL, apply `schema/*.sql`. |
| **Backend** | `cd backend && cargo run` (or `cargo run --release`). Serves API on `/v1/*` and frontend when `STATIC_DIR` points at it. |
| **Frontend** | No separate server; open http://localhost:3000 after the backend is running. |

For a single command that starts only the app (DB and migrations are manual):

```bash
export OPENAI_API_KEY=sk-...   # optional
cd backend && STATIC_DIR=../frontend cargo run
# then open http://localhost:3000
```
