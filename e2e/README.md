# E2E tests (`e2e/`)

Maps to `temp_mds/TEST_CASES.md`. Shared setup runs **once** per invocation:

- `BackendDockerStack` — `docker compose` from repo root, waits for `GET /health/ready`
- `BackendSharedAuth` — **one** backend registration per run (avoids per-IP register rate limit)
- `CliSharedSession` — **one** `prke register` per run (same rate limit)

## Run

```bash
# From repo root
python3 e2e/run_suites.py --suite all
python3 e2e/run_suites.py --suite backend_api
python3 e2e/run_suites.py --suite cli_only
python3 e2e/run_suites.py --cases case_2_1_2_login_invalid_returns_error
```

Optional:

- `--no-reset-db` — faster re-runs (same DB volume)
- `E2E_RUN_EXEC=1` plus `E2E_OPENAI_API_KEY` / `E2E_GEMINI_API_KEY` / `E2E_ANTHROPIC_API_KEY` — enable §8–10 execute tests

## Layout

| Path | Role |
|------|------|
| `common/` | Docker stack, HTTP client, PoW, key checksum, shared backend/CLI sessions |
| `suites/backend_api/` | Direct HTTP cases (`cases_*.py`) |
| `suites/cli_only/` | CLI-only cases (`cases_*.py`) |
| `run_suites.py` | Imports suite classes and runs `case_*` methods |

Add a new case: add `def case_foo(self):` to the right `cases_*.py` class (name prefix `case_`).
