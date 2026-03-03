# PromptKeeper Android SDK

Kotlin/Android SDK for the [Prompt Keeper](https://github.com/promptkeeper/promptkeeper) API. Supports **init** (in-memory API key), **setKey**, **setPrompt**, and **exec** (streaming). Register and obtain an API key outside this SDK; the SDK does not handle registration or login.

## Requirements

- Kotlin 1.9+
- JVM 11+ (or Android minSdk 24+ with coreLibraryDesugaring if needed)
- Coroutines and OkHttp (declared as dependencies)

## Installation (Maven)

### Maven Local (after publishing)

From the `sdks/android` directory (create the wrapper first with `gradle wrapper` if needed):

```bash
./gradlew publishToMavenLocal
```

In your app's `build.gradle.kts`:

```kotlin
repositories {
    mavenLocal()
}
dependencies {
    implementation("com.promptkeeper:android-sdk:1.0.0")
}
```

### Publish to your Maven repo

Configure `publishing.repositories` in `build.gradle.kts` (e.g. Maven Central or your private repo), then:

```bash
./gradlew publish
```

## Usage

### 1. Init (in-memory API key)

The API key is stored **in-memory only** for the current app process. It is not persisted.

```kotlin
// Option A: Initialize and keep the instance
val sdk = PromptKeeper.initialize(apiKey = "pk_your_key_here")

// Option B: Use constructor and hold reference
val sdk = PromptKeeper(apiKey = "pk_...")

// Option C: Init once, use getInstance() later
PromptKeeper.initialize(apiKey = "pk_...")
// ... later ...
PromptKeeper.getInstance()?.setKey(...)
```

### 2. Set key (store provider API key)

Stores a provider API key (e.g. OpenAI, Anthropic) on the server.

```kotlin
val response = sdk.setKey(rawSecret = "sk-...", provider = "openai")
// response: PutKeyResponse(versionId, createdAt, kmsKeyArn, fingerprint)
```

### 3. Set prompt (store prompt template)

Stores a prompt template for a named function.

```kotlin
val response = sdk.setPrompt(
    name = "customer_support",
    rawSecret = "Hello {{name}}! You asked: {{query}}",
    provider = "openai",
    preferredModel = "gpt-4o"
)
// response: PutPromptResponse(versionId, createdAt, kmsKeyArn, fingerprint)
```

### 4. Exec (streaming)

Executes a function and streams the LLM response as SSE chunks.

```kotlin
// From a coroutine or ViewModel
lifecycleScope.launch {
    sdk.exec(
        functionId = "default",
        variables = mapOf("name" to "Alice", "query" to "What is the return policy?"),
        provider = "anthropic"
    ).catch { e -> /* handle PromptKeeperException */ }
     .collect { chunk ->
        // chunk is raw SSE data (e.g. JSON from OpenAI/Anthropic stream)
        println(chunk)
     }
}
```

Errors from the server (e.g. function not found) are delivered as `PromptKeeperException.Server(message)`. HTTP errors as `PromptKeeperException.Http(statusCode, body)`.

### Exec response format: text, image, and video

SSE always delivers **text**: each chunk is the raw `data` line (a string). What that string contains depends on the provider and function:

| Use case | What you get | How to handle |
|----------|--------------|---------------|
| **Chat / completions** | JSON stream chunks (e.g. `choices[].delta.content`, Anthropic text deltas) | Parse JSON; concatenate or display text deltas. |
| **Image generation** (e.g. OpenAI DALL·E, GPT image models) | JSON string with `b64_json` (base64) or `url` | Parse chunk as JSON; decode `b64_json` to bytes or load image from `url`. |
| **Video** | Provider-specific JSON (often URL or base64) | Parse each chunk as JSON; use URL or decode base64 for playback. |

So `Flow<String>` is correct: the response is never raw binary over SSE. For images/video, the **payload is still a string** (JSON); your app parses it and extracts base64 or URL. Example for image:

```kotlin
sdk.exec(functionId = "image_gen", variables = mapOf("prompt" to "a cat")).collect { chunk ->
    val obj = kotlinx.serialization.json.Json.parseToJsonElement(chunk).jsonObject
    val b64 = obj["b64_json"]?.toString()?.trim('"')
    val url = obj["url"]?.toString()?.trim('"')
    if (b64 != null) {
        val bytes = android.util.Base64.decode(b64, android.util.Base64.DEFAULT)
        // BitmapFactory.decodeByteArray(bytes, 0, bytes.size), etc.
    }
    if (url != null) { /* load image from url */ }
}
```

## API summary

| Method | Description |
|--------|-------------|
| `PromptKeeper.initialize(apiKey)` | Init SDK; key in-memory only. Returns instance. |
| `PromptKeeper.getInstance()` | Returns instance set by `initialize`, or null. |
| `setKey(rawSecret, provider)` | POST /v1/keys — store provider API key. |
| `setPrompt(name, rawSecret, provider?, preferredModel?)` | POST /v1/prompts — store prompt template. |
| `exec(functionId, variables?, provider?, model?)` | POST /v1/execute — stream LLM response as `Flow<String>`. |

## Exceptions

- `PromptKeeperException.Http(statusCode, body)` — non-success HTTP response.
- `PromptKeeperException.Server(message)` — server error in SSE/JSON (e.g. `{"error":"..."}`).
- `PromptKeeperException.Network(cause)` — network/IO failure.

## License

See the root repository license.
