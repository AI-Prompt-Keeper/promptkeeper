# PromptKeeper Swift SDK

Swift package for the [Prompt Keeper](https://github.com/your-org/promptkeeper) backend. Use it to store provider API keys, prompt templates, and execute LLM functions with streaming responses.

**Requirements:** You must obtain an API key (e.g. via your backend's registration flow) before using this SDK. This package does not implement register or login.

## Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/your-org/promptkeeper-ios-sdk.git", from: "1.0.0")
],
targets: [
    .target(name: "YourApp", dependencies: ["PromptKeeper"])
]
```

Or in Xcode: **File → Add Package Dependencies** and enter the package URL.

**Local development:** Add a path dependency (from repo root: `path: "sdks/ios"`):

```swift
dependencies: [
    .package(path: "sdks/ios")
]
```

## Usage

### 1. Initialize (API key, in-memory only)

The API key is kept in memory only for the current app run. It is **not** persisted (no Keychain or UserDefaults).

```swift
import PromptKeeper

let sdk = PromptKeeper(apiKey: "pk_your_api_key_here")
```

### 2. Set key (store provider API key)

Store a provider API key (e.g. OpenAI, Anthropic) on the server.

```swift
let response = try await sdk.setKey(
    rawSecret: "sk-...",
    provider: "openai"
)
print(response.version_id, response.created_at)
```

### 3. Set prompt (store prompt template)

Store a Handlebars prompt template for a named function.

```swift
let response = try await sdk.setPrompt(
    name: "customer_support",
    rawSecret: "Hello {{name}}! You asked: {{query}}",
    provider: "openai",
    preferredModel: "gpt-4o"
)
```

### 4. Exec (streaming)

Execute a function and consume the LLM response stream.

```swift
let stream = sdk.exec(
    functionId: "customer_support",
    variables: ["name": "Alice", "query": "What is the return policy?"],
    provider: "anthropic"
)

for try await event in stream {
    if case .chunk(let data) = event {
        print(data)  // Provider-specific payload (e.g. SSE chunk)
    }
}
```

Errors from the server (e.g. function not found, provider error) are delivered as a thrown error when the stream sends an SSE event with `{ "error": "..." }`.

## API summary

| Method | Description |
|--------|-------------|
| `init(apiKey:)` | Initialize with API key (in-memory only). |
| `setKey(rawSecret:provider:)` | POST `/v1/keys` — store provider API key. |
| `setPrompt(name:rawSecret:provider:preferredModel:)` | POST `/v1/prompts` — store prompt template. |
| `exec(functionId:variables:provider:model:)` | POST `/v1/execute` — run function, returns `AsyncThrowingStream<ExecStreamEvent, Error>`. |

## Platforms

- iOS 15+
- macOS 12+

## License

See the repository license.
