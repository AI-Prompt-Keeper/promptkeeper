# Prompt Keeper CLI (prke / promptkeeper)

Minimalist CLI for testing the Secure AI Gateway. Built with Go, Cobra, and Viper. Produces native binaries for Linux, macOS, and Windows with no extra runtime dependencies.

## Install

```bash
cd cli
go build -o prke .
# Optional: install to PATH
cp prke /usr/local/bin/
ln -sf prke /usr/local/bin/promptkeeper
```

## Cross-compile

```bash
make build-all
# Outputs: bin/prke-{linux,darwin,windows}-{amd64,arm64}
```

## Config

By default, **`~/.prke-config.yaml` is not read**. **Base URL** resolution:

1. If **`--debug` and `--use-local-config`** are both set: use **`base_url`** from `~/.prke-config.yaml` when it is non-empty.
2. Otherwise, or if that value is empty or missing: **`PKRE_BASE_URL`** env (if set), else default **`https://api.promptkeeper.ai`**.

**API key**: system keyring only (never in the config file).

To use the local config file for `base_url` (e.g. local Docker testing), pass **both** flags:
```bash
prke --debug --use-local-config register ...
prke --debug --use-local-config exec my_prompt
```
When both are set, `~/.prke-config.yaml` is read and can be written when updating `base_url`. The API key is never stored in the config file.

Config file format:
```yaml
base_url: "https://api.promptkeeper.com"
```

## Commands

**Workspaces & client API keys** (see `docs/CLIENT_KEY_MANAGEMENT.md` and `backend/backend-specs.md`): the CLI keeps **per-workspace** management (`pk_mgt_live_...`) and execution (`pk_exe_live_...`) keys in the **OS secure store** on macOS/Linux (with a fallback to printing secrets if keyring is unavailable). Active workspace is stored in `~/.prke-state.yaml`. Registration creates a **personal** default workspace; login selects it and may require **`workspace mint-mgt`** because existing keys cannot be fetched from the API.

### 1. register \<email\> \<password\>

Register a new user. On success, stores the **management** API key for your **default (personal) workspace** in the OS secure store and prints a reminder (shown once).

```bash
prke register user@example.com securePassword123
```

### 2. login \<email\> \<password\>

Sign in with `POST /v1/auth/login`. Stores the **session token**, sets the active workspace to your **personal** workspace, and prints JSON (`token`, `expires_at`, `user`). If you have no management key stored locally for that workspace, run **`workspace mint-mgt`** (keys are not retrievable from the server). Omit email/password for the interactive form.

```bash
prke login user@example.com securePassword123
prke login   # guided: email + password
```

### 3. workspace (list \| switch \| create \| mint-mgt \| current)

Manage workspaces (`GET/POST /v1/workspaces`). Use **`workspace switch \<uuid\>`** before commands that need a specific workspace. If you have no stored management key for that workspace, run **`workspace mint-mgt`** after logging in.

```bash
prke workspace list
prke workspace switch 00000000-0000-0000-0000-000000000000
prke workspace create "My team"
prke workspace mint-mgt "New laptop"
prke workspace current
```

### 4. set prke_key \<key\>

Store a client key for the **active workspace** (management `pk_mgt_live_...` or execution `pk_exe_live_...`), or a session token. If no active workspace is set, uses a legacy single keyring slot.

```bash
prke set prke_key pk_mgt_live_...
```

### 5. mint key [label]

Mint an **execution-only** client API key via `POST /v1/auth/api-tokens`. Requires a **management** key (or session token) in the vault — not an execution-only key. The new key is printed once.

```bash
prke mint key
prke mint key "Mobile app"
```

### 6. store key \<provider\> \<api_key\>

Store a provider API key (OpenAI, Anthropic, etc.) in the gateway.

```bash
prke store key openai sk-xxxxx
```

### 7. store prompt \<prompt_title\> \<prompt_value|file_path\> [provider]

Store a prompt template. Second argument can be inline text or a file path. Use `--model` to set the preferred LLM model.

```bash
prke store prompt my_prompt "Hello {{name}}!" openai
prke store prompt my_prompt "Hello {{name}}!" openai --model gpt-4o
prke store prompt my_prompt ./prompt.txt
```

### 8. exec \<prompt_title\> [key=value...] [--provider provider] [--model model]

Execute a prompt with streaming output. Use `--model` to override the LLM model for this run.

```bash
prke exec my_prompt name=Alice query="What is X?"
prke exec default name=Bob --provider anthropic
prke exec default name=Bob --model gpt-4o
```

### 9. list prompts

List stored prompt **titles** (function names with a production deployment). Calls `GET /v1/list-prompts` with `surface=cli`. Works with **management** or **execution** client API keys.

```bash
prke list prompts
```

## Security

- Input validation: email, password, path traversal, length limits
- Paths: `..` rejected; file size limited to 64KB
- All errors printed to stderr
- **Secrets:** management/execution/session tokens are stored in the OS keychain (macOS) or secret service (Linux) when available; otherwise the CLI prints them to stdout with a warning so you can copy them elsewhere