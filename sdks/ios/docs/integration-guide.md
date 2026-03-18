# PromptKeeper iOS SDK — Integration Guide

**Audience:** AI agents integrating the PromptKeeper Swift package into an iOS/macOS app or another agent’s instructions.

**Summary:** The SDK talks to the Prompt Keeper backend (API key storage, prompt templates, and LLM execution). It is distributed via Swift Package Manager (SPM). All network calls are async and the API key is held in memory only for the current run.

---

## 1. Requirements

- **Platforms:** iOS 15+, macOS 12+
- **Swift:** 5.9+
- **Distribution:** Swift Package Manager only

---

## 2. Adding the dependency

In the app’s **Package.swift** (or Xcode: File → Add Package Dependencies), add:

```swift
dependencies: [
    .package(url: "https://github.com/AI-Prompt-Keeper/promptkeeper", from: "1.0.0"),
],
targets: [
    .target(
        name: "YourTarget",
        dependencies: [
            .product(name: "PromptKeeper", package: "promptkeeper"),
        ]
    ),
]
```

- **Product name** (what you `import`): `"PromptKeeper"` — from the library name in Package.swift.
- **Package identity** (the `package:` argument): derived by SPM from the repo URL; for a URL ending in `promptkeeper` it is `"promptkeeper"` (lowercase). Use that exact string if your URL matches.

If the package lives in a subfolder (e.g. `sdks/ios`), use the repository URL that resolves to the repo root and set the package path or use a git ref that includes the `Package.swift` at the resolved path. In Xcode, add the package by URL and select the `PromptKeeper` library product.

---

## 3. Initialization

Create a single client with your Prompt Keeper API key (obtain via your backend/registration; the SDK does not persist it).

```swift
import PromptKeeper

let client = PromptKeeper(apiKey: "pk_your_api_key")
```

- The API key is kept in memory for the lifetime of the `PromptKeeper` instance only.
- No Keychain, UserDefaults, or file persistence.

---

## 4. Storing a provider API key (`setKey`)

Store a provider key (e.g. OpenAI, Anthropic) on the Prompt Keeper server. The raw secret is sent to the server only; the SDK does not store it locally.

**Signature:**

```swift
func setKey(rawSecret: String, provider: String) async throws -> PutKeyResponse
```

**Parameters:**

| Parameter   | Type   | Description                                  |
|------------|--------|----------------------------------------------|
| `rawSecret`| String | Raw API key (e.g. `sk-...`). Sent to server. |
| `provider` | String | Provider id, e.g. `"openai"`, `"anthropic"`. |

**Returns:** `PutKeyResponse` with Swift properties: `versionId`, `createdAt`, `kmsKeyArn`, `fingerprint` (all from server; optional fields are `String?`).

**Example:**

```swift
let response = try await client.setKey(rawSecret: "sk-...", provider: "openai")
print(response.versionId, response.createdAt)
```

---

## 5. Storing a prompt template (`setPrompt`)

Register a prompt template for a named function. The template can use Handlebars-style variables. The SDK does not persist the template locally.

**Signature:**

```swift
func setPrompt(
    name: String,
    rawSecret: String,
    provider: String? = nil,
    preferredModel: String? = nil
) async throws -> PutPromptResponse
```

**Parameters:**

| Parameter       | Type   | Description                                                |
|----------------|--------|------------------------------------------------------------|
| `name`         | String | Function/prompt name (e.g. `"customer_support"`).          |
| `rawSecret`    | String | Raw prompt body (e.g. Handlebars). Sent to server only.   |
| `provider`     | String?| Default provider (e.g. `"openai"`).                        |
| `preferredModel`| String?| Default model (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-20240620"`). |

**Returns:** `PutPromptResponse` with `versionId`, `createdAt`, `kmsKeyArn`, `fingerprint`.

**Example:**

```swift
let response = try await client.setPrompt(
    name: "customer_support",
    rawSecret: "You are a support agent. User: {{user_message}}",
    provider: "openai",
    preferredModel: "gpt-4o"
)
```

---

## 6. Executing a stored function (`exec`) — streaming

Run a **stored** function by id: the server resolves the prompt template, injects variables, calls the configured LLM, and streams the response. Use this when you have already stored a prompt via `setPrompt`.

**Signature:**

```swift
func exec(
    functionId: String,
    variables: [String: String]? = nil,
    provider: String? = nil,
    model: String? = nil
) -> AsyncThrowingStream<ExecStreamEvent, Error>
```

**Parameters:**

| Parameter   | Type            | Description                                           |
|------------|-----------------|-------------------------------------------------------|
| `functionId`| String          | Function id (e.g. `"default"`, `"customer_support_reply"`). |
| `variables`| [String: String]?| Handlebars variables. Default `nil` → empty.          |
| `provider` | String?         | Override provider (e.g. `"openai"`, `"anthropic"`).   |
| `model`    | String?         | Override model.                                       |

**Returns:** `AsyncThrowingStream<ExecStreamEvent, Error>`. Each element is:

- `ExecStreamEvent.chunk(String)` — one SSE data payload (provider-specific; e.g. JSON chunk from OpenAI/Anthropic stream).

The stream throws on network or HTTP errors. The server may also send an error in the SSE stream as JSON `{ "error": "..." }`; the SDK parses that and throws `PromptKeeperError.serverError(message)`.

**Example — consume stream:**

```swift
let stream = client.exec(
    functionId: "customer_support_reply",
    variables: ["user_message": "I need help with my order"]
)
do {
    for try await event in stream {
        if case .chunk(let data) = event {
            // data is the raw chunk string (e.g. JSON). Parse per provider.
            print(data)
        }
    }
} catch {
    print("Execution failed: \(error)")
}
```

**Example — with error type handling:**

```swift
do {
    for try await event in client.exec(functionId: "default", variables: nil) {
        if case .chunk(let data) = event { process(data) }
    }
} catch let err as PromptKeeperError {
    print(err.message)
} catch {
    print(error)
}
```

---

## 6b. Executing a raw prompt (`execPrompt`) — streaming

Run a **raw text prompt** without storing it as a function first. The backend receives the prompt and optional variables/provider/model and streams the response. Use endpoint `POST /v1/execute-raw` (same SSE shape as `exec`); use this for ad-hoc or one-off calls.

**Signature:**

```swift
func execPrompt(
    prompt: String,
    variables: [String: String]? = nil,
    provider: String? = nil,
    model: String? = nil
) -> AsyncThrowingStream<ExecStreamEvent, Error>
```

**Parameters:**

| Parameter   | Type            | Description                                           |
|------------|-----------------|-------------------------------------------------------|
| `prompt`   | String          | Raw prompt text to execute (not stored on the server). |
| `variables`| [String: String]?| Optional Handlebars variables. Default `nil` → empty. |
| `provider` | String?         | Preferred provider (e.g. `"openai"`, `"anthropic"`, `"gemini"`). Backend requires a provider for raw execution. |
| `model`    | String?         | Model override.                                       |

**Returns:** Same as `exec`: `AsyncThrowingStream<ExecStreamEvent, Error>`. Each element is `ExecStreamEvent.chunk(String)` (provider payload). Errors are thrown or delivered in SSE `{ "error": "..." }`.

**Example:**

```swift
let stream = client.execPrompt(
    prompt: "You are a helpful assistant. Reply briefly: What is 2+2?",
    variables: nil,
    provider: "openai",
    model: nil
)
for try await event in stream {
    if case .chunk(let data) = event { process(data) }
}
```

---

## 7. Error handling

All throwing APIs use the SDK’s error type or standard Swift errors.

**`PromptKeeperError`:**

| Case | Meaning |
|------|--------|
| `httpStatus(Int, body: Data)` | HTTP response had a non-success status; associated body is the response data. |
| `serverError(String)` | Server sent an error in the stream (e.g. JSON `{ "error": "..." }`). |

**Common usage:**

- Use `error.message` for a single human-readable string (logging or UI).
- Use the enum cases when you need to branch on HTTP status or server message.

```swift
do {
    try await client.setKey(rawSecret: key, provider: "openai")
} catch let e as PromptKeeperError {
    switch e {
    case .httpStatus(let code, let body):
        print("HTTP \(code): \(String(data: body, encoding: .utf8) ?? "")")
    case .serverError(let msg):
        print("Server error: \(msg)")
    }
}
```

---

## 8. Public types reference

| Type | Description |
|------|-------------|
| `PromptKeeper` | Main client. Create with `init(apiKey:)`. Methods: `setKey`, `setPrompt`, `exec`, `execPrompt`. |
| `PutKeyResponse` | `versionId`, `createdAt`, `kmsKeyArn`, `fingerprint`. |
| `PutPromptResponse` | Same shape as `PutKeyResponse`. |
| `ExecStreamEvent` | `.chunk(String)` — one streamed data chunk. |
| `PromptKeeperError` | `httpStatus(Int, body: Data)`, `serverError(String)`; `message: String`. |

All of these are `Sendable` where applicable. Use from any async context (MainActor, background, or unstructured tasks).

---

## 9. Concurrency and lifecycle

- **Async only:** `setKey`, `setPrompt`, and the `exec` / `execPrompt` streams are async; call from `async` functions or `Task`.
- **No persistence:** The SDK does not write the API key or secrets to disk. Create a new `PromptKeeper(apiKey:)` each run if the key is provided at launch.
- **Stream cancellation:** When the consumer stops iterating the `AsyncThrowingStream` from `exec` or `execPrompt`, the underlying request is cancelled.

Use this guide to add the package, create a client, call `setKey`/`setPrompt`/`exec`/`execPrompt`, and handle `PromptKeeperError` and stream consumption in agent or app code.
