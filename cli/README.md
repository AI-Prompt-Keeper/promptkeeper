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

Config file: `~/.pv-config.yaml`

```yaml
base_url: "http://localhost:3000"
vault_access_token: "pk_..."   # API key (also stored in system keyring when possible)
```

- **base_url**: API base URL (default: `http://localhost:3000`)
- **vault_access_token**: Your API key. Stored in system keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service) when available; otherwise in this file.

## Commands

### 1. register \<email\> \<password\>

Register a new user. On success, stores the API key in the system vault and prints a reminder to save it.

```bash
prke register user@example.com securePassword123
```

### 2. set prke_key \<key\>

Store the API key for subsequent requests.

```bash
prke set prke_key pk_xxxxxxxxxxxx
```

### 3. store key \<provider\> \<api_key\>

Store a provider API key (OpenAI, Anthropic, etc.) in the gateway.

```bash
prke store key openai sk-xxxxx
```

### 4. store prompt \<prompt_title\> \<prompt_value|file_path\> [provider]

Store a prompt template. Second argument can be inline text or a file path. Use `--model` to set the preferred LLM model.

```bash
prke store prompt my_prompt "Hello {{name}}!" openai
prke store prompt my_prompt "Hello {{name}}!" openai --model gpt-4o
prke store prompt my_prompt ./prompt.txt
```

### 5. exec \<prompt_title\> [key=value...] [--provider provider] [--model model]

Execute a prompt with streaming output. Use `--model` to override the LLM model for this run.

```bash
prke exec my_prompt name=Alice query="What is X?"
prke exec default name=Bob --provider anthropic
prke exec default name=Bob --model gpt-4o
```

## Security

- Input validation: email, password, path traversal, length limits
- Paths: `..` rejected; file size limited to 64KB
- All errors printed to stderr
