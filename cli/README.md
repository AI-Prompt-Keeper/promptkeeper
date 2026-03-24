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

**Client API keys** (see `docs/CLIENT_KEY_MANAGEMENT.md` in the repo): registration returns a **management** key (`pk_mgt_live_...`). You can **mint** an **execution-only** key (`pk_exe_live_...`) for embeds and automation that should only call `POST /v1/execute`. Execution-only keys get **403** on storing provider keys, prompts, or minting further tokens.

### 1. register \<email\> \<password\>

Register a new user. On success, stores the **management** API key in the system vault and prints a reminder to save it (shown once).

```bash
prke register user@example.com securePassword123
```

### 2. set prke_key \<key\>

Store the Prompt Keeper client API key for subsequent requests (management or execution-only).

```bash
prke set prke_key pk_mgt_live_...
```

### 3. mint key [label]

Mint an **execution-only** client API key via `POST /v1/auth/api-tokens`. Requires a **management** key (or session token) in the vault — not an execution-only key. The new key is printed once.

```bash
prke mint key
prke mint key "Mobile app"
```

### 4. store key \<provider\> \<api_key\>

Store a provider API key (OpenAI, Anthropic, etc.) in the gateway.

```bash
prke store key openai sk-xxxxx
```

### 5. store prompt \<prompt_title\> \<prompt_value|file_path\> [provider]

Store a prompt template. Second argument can be inline text or a file path. Use `--model` to set the preferred LLM model.

```bash
prke store prompt my_prompt "Hello {{name}}!" openai
prke store prompt my_prompt "Hello {{name}}!" openai --model gpt-4o
prke store prompt my_prompt ./prompt.txt
```

### 6. exec \<prompt_title\> [key=value...] [--provider provider] [--model model]

Execute a prompt with streaming output. Use `--model` to override the LLM model for this run.

```bash
prke exec my_prompt name=Alice query="What is X?"
prke exec default name=Bob --provider anthropic
prke exec default name=Bob --model gpt-4o
```

### 7. list prompts

List stored prompt **titles** (function names with a production deployment). Calls `GET /v1/list-prompts` with `surface=cli`. Works with **management** or **execution** client API keys.

```bash
prke list prompts
```

## Security

- Input validation: email, password, path traversal, length limits
- Paths: `..` rejected; file size limited to 64KB
- All errors printed to stderr