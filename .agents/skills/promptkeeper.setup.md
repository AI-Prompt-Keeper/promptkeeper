# Skill: PromptKeeper (prke)
ID: ai.promptkeeper
Version: 0.1.0-alpha
Description: PromptKeeper (AI-secrets firewall) installation, setup and operation guide

# PromptKeeper Agent Skill — Installation, Setup, and Integration

## Purpose

This skill enables an AI coding agent to:

1. Install PromptKeeper CLI and SDKs.
2. Register a user.
3. Obtain and configure a **management key**.
4. Store provider API keys and prompts.
5. Integrate PromptKeeper execution into an existing mobile or backend application.

This document is procedural and optimized for autonomous execution.

---

## Conceptual Model (Required Understanding)

PromptKeeper is a **dual-key encrypted prompt vault**.

Two key types exist:

| Key            | Prefix    | Capability                                                |
| -------------- | --------- | --------------------------------------------------------- |
| Management Key | `pk_mgt_` | Create/update prompts, store secrets, mint execution keys |
| Execution Key  | `pk_exe_` | Execute prompts only (no mutations)                       |

Agents MUST follow this rule:

> Management keys are NEVER embedded into application source code.

Execution keys MAY be embedded into runtime environments.

---

## Preconditions

Before executing this skill, ensure:

* Internet connectivity is available.
* A terminal environment exists.
* Go toolchain OR package manager installation is allowed.
* The user consents to creating a PromptKeeper account.

Optional:

* Android Studio or Xcode available for mobile integration.

---

## Phase 1 — Install PromptKeeper CLI

### Option A (Preferred) — Package Managers (If Available)

macOS:

```bash
brew tap ai-prompt-keeper/homebrew-tap
brew install prke
```

Linux:

```bash
snap install prke
```
-----

### Option B — Build from Source

```bash
git clone https://github.com/AI-Prompt-Keeper/promptkeeper.git
cd promptkeeper/cli
go build -o prke .
```
-----

### Option C — Go Install

```bash
go install github.com/promptkeeper/cli@latest
```
-----

Verify installation:

```bash
prke --help
```

Expected output - help output displaying available commands

If the command is not available - stop and report failure.

-----

## Phase 2 — Create an account

Registration creates the vault and returns a **management key**.

**This step requires a valid email and a password. Ask user for input.**

Execute:

```bash
prke register <email> '<secure-password>'
```

Behavior:

1. CLI solves Proof-of-Work challenge.
2. Backend creates encrypted workspace.
3. Management key is returned ONCE.

Agent responsibilities:

* Capture management key securely.
* Do NOT print key in logs unless explicitly requested.
* Store locally via CLI vault.

Expected output includes:

```
IMPORTANT: Store your API key securely
```

CLI stores management key internally, so following commands have it attached automatically.

---

## Phase 3 — Store Provider Credentials (Optional)

This step is optional. User might want to do it manually.
To proceed with this step, you need to obtain LLM Provider API keys.

Store LLM provider API key.

Example:

```bash
prke store key openai sk-xxxxxxxx
```

Supported providers:
1. OpenAI (openai)
2. Anthropic (anthropic)
3. Google (gemini) (text-only)

Expected result:

```
✓ secret stored
```

Agent must verify successful response before continuing.

---

## Phase 4 — Store Prompt Template (Optional)

This step is optional. User might want to do it manually.
To proceed with this step, you need to obtain a prompt and a target provider.

Example:

```bash
prke store prompt greeting \
"Hello {{name}}! You asked: {{query}}" \
openai --model gpt-4o
```

Validation step:

```bash
prke list prompts
```

Prompt name must appear.

---

## Phase 5 — Mint Execution Key

Create runtime-safe credential:

```bash
prke mint key
```

Output contains:

```
pk_exe_live_...
```

Agent rules:

* Treat execution key as deployable credential.
* Do NOT expose management key beyond setup phase.

Store execution key in a secure storage.

---

## Phase 6 — Test Execution

Run:

```bash
prke exec greeting name=Alice query="status?"
```

Expected behavior:

* Streaming response returned.
* No mutation permissions allowed.

If execution succeeds → integration ready.

---

## Phase 7 — Mobile Integration

### Android (Gradle)
Refer to https://github.com/AI-Prompt-Keeper/promptkeeper/blob/main/sdks/android/README.md for detailed Android instructions.

---

### iOS (Swift Package Manager)
Refer to https://github.com/AI-Prompt-Keeper/promptkeeper/blob/main/sdks/ios/README.md for detailed iOS instructions.

---

## Operational Rules for Agents

Agents MUST:

* Use management keys only during setup.
* Switch to execution keys for runtime testing.
* Avoid committing any keys to repositories.
* Prefer environment variables for execution keys.

Recommended:

```
PROMPTKEEPER_API_KEY=pk_exe_live_...
```

---

## Failure Handling

If any command returns:

* HTTP 403 → wrong key scope.
* HTTP 401 → invalid key.
* Network error → retry once, then report.

Never retry registration automatically.

---

## Security Constraints

Agents must assume:

* Execution keys may leak.
* Management keys must remain local.
* Vault data is ciphertext at rest.

Agents must NOT:

* Attempt to extract stored secrets.
* Log provider API keys.
* Store management keys in app code.

---

## Completion Criteria

Skill execution is successful when:

* CLI installed and verified.
* Workspace registered.
* Provider key stored.
* Prompt stored.
* Execution key minted.
* Prompt successfully executed.
* Mobile SDK integrated and runnable.

