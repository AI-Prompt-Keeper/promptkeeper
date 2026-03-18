# PromptKeeper Android SDK — Integration Guide

**Audience:** AI agents integrating the PromptKeeper Android/Kotlin library into an Android app or another agent’s instructions.

**Summary:** The SDK talks to the Prompt Keeper backend (API key storage, prompt templates, and LLM execution) at **https://api.promptkeeper.ai**. It is distributed via Maven (Maven Central or `mavenLocal()`). All network calls use Kotlin coroutines; the API key is held in memory only for the current process.

---

## 1. Requirements

- **Kotlin:** 1.9+
- **JVM / Android:** JVM 11+ or Android minSdk 24+ (with coreLibraryDesugaring if using Java 11 APIs on older minSdk)
- **Distribution:** Maven only (`ai.promptkeeper:android-sdk`)
- **Dependencies (transitive):** OkHttp, kotlinx-coroutines, kotlinx-serialization-json

---

## 2. Adding the dependency

In the app’s **build.gradle.kts** (or **build.gradle**), add the repository and dependency.

**Kotlin DSL (build.gradle.kts):**

```kotlin
repositories {
    mavenCentral()
    // mavenLocal() // if using a locally published build
}

dependencies {
    implementation("ai.promptkeeper:android-sdk:1.0.0")
}
```

**Groovy (build.gradle):**

```groovy
repositories {
    mavenCentral()
}

dependencies {
    implementation 'ai.promptkeeper:android-sdk:1.0.0'
}
```

**Maven coordinates:**

| Coordinate | Value |
|------------|--------|
| Group ID   | `ai.promptkeeper` |
| Artifact ID| `android-sdk`     |
| Version    | Use latest from Maven Central (e.g. `1.0.0`) |

**Kotlin package (for imports):** `ai.promptkeeper.sdk`

---

## 3. Initialization

Create a client with the Prompt Keeper API key (obtain via your backend/registration; the SDK does not persist it).

```kotlin
import ai.promptkeeper.sdk.PromptKeeper

// Option A: Constructor and hold reference
val sdk = PromptKeeper(apiKey = "pk_your_api_key")

// Option B: Initialize once, use getInstance() later
PromptKeeper.initialize(apiKey = "pk_your_api_key")
val sdk = PromptKeeper.getInstance()!!
```

- The API key is kept in memory for the lifetime of the process only.
- No SharedPreferences, DataStore, or file persistence.
- Base URL is fixed to `https://api.promptkeeper.ai` (not configurable in the public API).

---

## 4. Storing a provider API key (`setKey`)

Store a provider key (e.g. OpenAI, Anthropic) on the Prompt Keeper server. The raw secret is sent to the server only; the SDK does not store it locally.

**Signature:**

```kotlin
suspend fun setKey(rawSecret: String, provider: String): PutKeyResponse
```

**Parameters:**

| Parameter   | Type   | Description                                  |
|------------|--------|----------------------------------------------|
| `rawSecret`| String | Raw API key (e.g. `sk-...`). Sent to server. |
| `provider` | String | Provider id, e.g. `"openai"`, `"anthropic"`. |

**Returns:** `PutKeyResponse` with `versionId: Long`, `createdAt: String`, `kmsKeyArn: String`, `fingerprint: String`.

**Example (inside a coroutine):**

```kotlin
import ai.promptkeeper.sdk.PromptKeeper
import ai.promptkeeper.sdk.model.PutKeyResponse

// e.g. in ViewModel or use case
viewModelScope.launch {
    val response: PutKeyResponse = sdk.setKey(
        rawSecret = "sk-...",
        provider = "openai"
    )
    // response.versionId, response.createdAt, etc.
}
```

---

## 5. Storing a prompt template (`setPrompt`)

Register a prompt template for a named function. The template can use Handlebars-style variables. The SDK does not persist the template locally.

**Signature:**

```kotlin
suspend fun setPrompt(
    name: String,
    rawSecret: String,
    provider: String? = null,
    preferredModel: String? = null
): PutPromptResponse
```

**Parameters:**

| Parameter        | Type   | Description                                                       |
|------------------|--------|-------------------------------------------------------------------|
| `name`           | String | Function/prompt name (e.g. `"customer_support"`).                |
| `rawSecret`      | String | Raw prompt body (e.g. Handlebars). Sent to server only.            |
| `provider`       | String?| Default provider (e.g. `"openai"`).                               |
| `preferredModel` | String?| Default model (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-20240620"`).   |

**Returns:** `PutPromptResponse` with `versionId`, `createdAt`, `kmsKeyArn`, `fingerprint`.

**Example:**

```kotlin
viewModelScope.launch {
    val response = sdk.setPrompt(
        name = "customer_support",
        rawSecret = "You are a support agent. User: {{user_message}}",
        provider = "openai",
        preferredModel = "gpt-4o"
    )
}
```

---

## 6. Executing a stored function (`exec`, streaming)

Execute a stored function: the backend resolves the prompt template by id, injects variables, calls the configured LLM, and streams the response as Server-Sent Events. Each emitted value is the raw SSE `data` payload (a string; often JSON).

**Signature:**

```kotlin
fun exec(
    functionId: String,
    variables: Map<String, String> = emptyMap(),
    provider: String? = null,
    model: String? = null
): Flow<String>
```

**Parameters:**

| Parameter    | Type                | Description                                              |
|-------------|---------------------|----------------------------------------------------------|
| `functionId`| String              | Stored function identifier (e.g. `"default"`, `"customer_support_reply"`). |
| `variables` | Map<String, String> | Optional template variables (Handlebars). Default: empty. |
| `provider`  | String?             | Preferred provider (e.g. `"openai"`, `"anthropic"`).     |
| `model`     | String?             | Model override.                                         |

**Returns:** `Flow<String>`. Each emission is one SSE `data` chunk (string). On server error (e.g. function not found), the flow throws `PromptKeeperException.Server(message)`.

**Example:**

```kotlin
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.collect

viewModelScope.launch {
    sdk.exec(
        functionId = "customer_support",
        variables = mapOf("name" to "Alice", "query" to "What is the return policy?"),
        provider = "anthropic"
    )
        .catch { e -> /* handle PromptKeeperException */ }
        .collect { chunk ->
            // chunk is raw SSE data (e.g. JSON from provider stream)
            println(chunk)
        }
}
```

**Exec response format (for agents):**

- **Chat/completions:** Chunks are typically JSON (e.g. OpenAI `choices[].delta.content`, Anthropic text deltas). Parse JSON and concatenate or display.
- **Image generation:** Chunk is still a string (JSON with `b64_json` or `url`). Parse JSON; decode base64 or load from URL.
- **Video:** Provider-specific JSON (URL or base64). Parse each chunk accordingly.

---

## 7. Executing a raw prompt (`execPrompt`, streaming)

Run a **raw text prompt** without storing a function. The SDK calls the backend `POST /v1/execute-raw`; the prompt is sent directly to the provider and nothing is stored. Same SSE streaming semantics as `exec`.

**Signature:**

```kotlin
fun execPrompt(
    prompt: String,
    variables: Map<String, String> = emptyMap(),
    provider: String,
    model: String? = null
): Flow<String>
```

Calls backend POST /v1/execute-raw. Parameters: prompt, variables (optional), provider (required), model (optional).
---

## 8. Exceptions

All exceptions extend `PromptKeeperException` (package `ai.promptkeeper.sdk`).

| Type                          | When |
|-------------------------------|------|
| `PromptKeeperException.Http`   | Non-success HTTP response. Fields: `statusCode: Int`, `body: ByteArray?`. |
| `PromptKeeperException.Server`| Server returned an error in SSE/JSON (e.g. `{"error":"function not found"}`). Field: `message: String`. |
| `PromptKeeperException.Network`| Network or I/O failure. Wraps cause. |

**Example handling:**

```kotlin
try {
    sdk.setKey(rawSecret = "sk-...", provider = "openai")
} catch (e: PromptKeeperException.Server) {
    // e.message
} catch (e: PromptKeeperException.Http) {
    // e.statusCode, e.body
} catch (e: PromptKeeperException.Network) {
    // e.cause
}
```

---

## 9. API summary (quick reference for agents)

| Method / API | Description |
|--------------|-------------|
| `PromptKeeper(apiKey: String)` | Create client; key in-memory only. |
| `PromptKeeper.initialize(apiKey: String)` | Init and set default instance; returns instance. |
| `PromptKeeper.getInstance()` | Returns instance set by `initialize`, or null. |
| `setKey(rawSecret, provider)` | `suspend`. POST /v1/keys. Returns `PutKeyResponse`. |
| `setPrompt(name, rawSecret, provider?, preferredModel?)` | `suspend`. POST /v1/prompts. Returns `PutPromptResponse`. |
| `exec(functionId, variables?, provider?, model?)` | Returns `Flow<String>`. POST /v1/execute (SSE). Stored function. |
| `execPrompt(prompt, variables?, provider?, model?)` | Returns `Flow<String>`. POST /v1/execute-raw (SSE). Raw prompt, no stored function. |

**Imports:**

```kotlin
import ai.promptkeeper.sdk.PromptKeeper
import ai.promptkeeper.sdk.PromptKeeperException
import ai.promptkeeper.sdk.model.PutKeyResponse
import ai.promptkeeper.sdk.model.PutPromptResponse
```

---

## 10. Backend and registration

- **Base URL:** https://api.promptkeeper.ai (hardcoded in SDK).
- **Auth:** All requests use the API key as `Authorization: Bearer <key>` and `X-API-Key: <key>`.
- **Registration / login:** Not implemented in the SDK. The app or another service must obtain the API key (e.g. `pk_...`) and pass it into the SDK.
