//
//  PromptKeeper.swift
//  PromptKeeper
//
//  Swift SDK for Prompt Keeper backend. Requires an API key (obtain via registration outside this SDK).
//

import Foundation

/// Main entry point for the Prompt Keeper SDK.
/// API key is kept in-memory only for the current app run; it is not persisted.
public final class PromptKeeper {

    /// Default production API base URL. Isolated here to avoid force-unwrap in public API.
    private static let defaultBaseURL: URL = URL(string: "https://api.promptkeeper.ai")!

    private let baseURL: URL
    private let apiKeyHolder: APIKeyHolder
    private let session: URLSession

    /// Creates and configures the SDK with an API key.
    /// - Parameter apiKey: API key obtained from your backend (e.g. after registration). Stored in-memory only for this app run.
    public init(apiKey: String) {
        self.baseURL = Self.defaultBaseURL
        self.apiKeyHolder = APIKeyHolder(apiKey: apiKey)
        self.session = URLSession.shared
    }

    /// Internal initializer for testing (inject base URL and session).
    init(apiKey: String, baseURL: URL, session: URLSession, apiKeyHolder: APIKeyHolder? = nil) {
        self.baseURL = baseURL
        self.apiKeyHolder = apiKeyHolder ?? APIKeyHolder(apiKey: apiKey)
        self.session = session
    }

    // MARK: - Set key (store provider API key)

    /// Stores a provider API key (e.g. OpenAI, Anthropic) on the server.
    /// - Parameters:
    ///   - rawSecret: Raw API key (e.g. `sk-...`). Not persisted by the SDK; sent only to the server.
    ///   - provider: Provider name (e.g. `"openai"`, `"anthropic"`).
    ///   - surface: Optional client surface tag for backend analytics. Default: `"ios"`.
    /// - Returns: Put key response with `version_id`, `created_at`, etc.
    public func setKey(
        rawSecret: String,
        provider: String,
        surface: String? = "ios"
    ) async throws -> PutKeyResponse {
        let endpoint = baseURL.appendingPathComponent("v1/keys")
        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setAuth(apiKeyHolder.apiKey)
        request.httpBody = try JSONEncoder().encode(
            PutKeyRequest(raw_secret: rawSecret, provider: provider, surface: surface)
        )

        let (data, response) = try await session.data(for: request)
        try Self.validateHTTP(response: response, data: data, expectedStatus: 201)
        return try JSONDecoder().decode(PutKeyResponse.self, from: data)
    }

    // MARK: - Set prompt (store prompt template)

    /// Stores a prompt template for a named function.
    /// - Parameters:
    ///   - name: Function/prompt name (e.g. `"customer_support"`).
    ///   - rawSecret: Raw prompt template (e.g. Handlebars). Not persisted by the SDK.
    ///   - provider: Optional default provider (e.g. `"openai"`).
    ///   - preferredModel: Optional default model (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-20240620"`).
    ///   - surface: Optional client surface tag for backend analytics. Default: `"ios"`.
    /// - Returns: Put prompt response with `version_id`, `created_at`, etc.
    public func setPrompt(
        name: String,
        rawSecret: String,
        provider: String? = nil,
        preferredModel: String? = nil,
        surface: String? = "ios"
    ) async throws -> PutPromptResponse {
        let endpoint = baseURL.appendingPathComponent("v1/prompts")
        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setAuth(apiKeyHolder.apiKey)
        let body = PutPromptRequest(
            name: name,
            raw_secret: rawSecret,
            provider: provider,
            preferred_model: preferredModel,
            surface: surface
        )
        request.httpBody = try JSONEncoder().encode(body)

        let (data, response) = try await session.data(for: request)
        try Self.validateHTTP(response: response, data: data, expectedStatus: 201)
        return try JSONDecoder().decode(PutPromptResponse.self, from: data)
    }

    // MARK: - Execute (streaming)

    /// Executes a function: resolves prompt, injects variables, calls the configured LLM, and streams the response.
    /// - Parameters:
    ///   - functionId: Function identifier (e.g. `"default"`, `"customer_support_reply"`).
    ///   - variables: Optional map of variable names to values (Handlebars). Default: empty.
    ///   - provider: Optional preferred provider (e.g. `"openai"`, `"anthropic"`).
    ///   - model: Optional model override.
    ///   - surface: Optional client surface tag for backend analytics. Default: `"ios"`.
    /// - Returns: An async sequence of SSE events. Each event's `data` contains provider payload (e.g. stream chunk). On error, a single event may contain JSON `{ "error": "..." }`.
    public func exec(
        functionId: String,
        variables: [String: String]? = nil,
        provider: String? = nil,
        model: String? = nil,
        surface: String? = "ios"
    ) -> AsyncThrowingStream<ExecStreamEvent, Error> {
        AsyncThrowingStream { continuation in
            var task: Task<Void, Never>? = Task {
                do {
                    let endpoint = baseURL.appendingPathComponent("v1/execute")
                    var request = URLRequest(url: endpoint)
                    request.httpMethod = "POST"
                    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
                    request.setAuth(apiKeyHolder.apiKey)
                    let body = ExecuteRequest(
                        function_id: functionId,
                        variables: variables ?? [:],
                        provider: provider,
                        model: model,
                        surface: surface
                    )
                    request.httpBody = try JSONEncoder().encode(body)

                    let (bytes, response) = try await session.bytes(for: request)
                    if let http = response as? HTTPURLResponse, http.statusCode != 200 {
                        var collected = Data()
                        for try await b in bytes { collected.append(b) }
                        throw PromptKeeperError.httpStatus(http.statusCode, body: collected)
                    }

                    try await StreamReader.readSSEBlocks(from: bytes) { block in
                        let events = SSEParser.parse(block)
                        for event in events {
                            guard let data = event.data, !data.isEmpty else { continue }
                            continuation.yield(.chunk(data))
                            if let err = SSEParser.parseErrorPayload(data) {
                                throw PromptKeeperError.serverError(err)
                            }
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in
                task?.cancel()
                task = nil
            }
        }
    }

    // MARK: - Execute with prompt (streaming)

    /// Executes a raw text prompt without storing it as a function first.
    /// Uses `POST /v1/execute-raw` with a `prompt` field (no stored function).
    /// Useful for ad-hoc calls where you do not want to manage prompt versions on the server.
    /// - Parameters:
    ///   - prompt: Raw prompt text to execute (not stored on the backend).
    ///   - variables: Optional map of variable names to values (Handlebars). Default: empty.
    ///   - provider: Optional preferred provider (e.g. `"openai"`, `"anthropic"`, `"gemini"`).
    ///   - model: Optional model override.
    ///   - surface: Optional client surface tag for backend analytics. Default: `"ios"`.
    /// - Returns: An async sequence of SSE events. Same semantics as `exec(functionId:...)`; each event's `data` contains provider payload. On error, a single event may contain JSON `{ "error": "..." }`.
    public func execPrompt(
        prompt: String,
        variables: [String: String]? = nil,
        provider: String? = nil,
        model: String? = nil,
        surface: String? = "ios"
    ) -> AsyncThrowingStream<ExecStreamEvent, Error> {
        AsyncThrowingStream { continuation in
            var task: Task<Void, Never>? = Task {
                do {
                    let endpoint = baseURL.appendingPathComponent("v1/execute-raw")
                    var request = URLRequest(url: endpoint)
                    request.httpMethod = "POST"
                    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
                    request.setAuth(apiKeyHolder.apiKey)
                    let body = ExecutePromptRequest(
                        prompt: prompt,
                        variables: variables ?? [:],
                        provider: provider,
                        model: model,
                        surface: surface
                    )
                    request.httpBody = try JSONEncoder().encode(body)

                    let (bytes, response) = try await session.bytes(for: request)
                    if let http = response as? HTTPURLResponse, http.statusCode != 200 {
                        var collected = Data()
                        for try await b in bytes { collected.append(b) }
                        throw PromptKeeperError.httpStatus(http.statusCode, body: collected)
                    }

                    try await StreamReader.readSSEBlocks(from: bytes) { block in
                        let events = SSEParser.parse(block)
                        for event in events {
                            guard let data = event.data, !data.isEmpty else { continue }
                            continuation.yield(.chunk(data))
                            if let err = SSEParser.parseErrorPayload(data) {
                                throw PromptKeeperError.serverError(err)
                            }
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in
                task?.cancel()
                task = nil
            }
        }
    }

    private static func validateHTTP(response: URLResponse?, data: Data, expectedStatus: Int) throws {
        guard let http = response as? HTTPURLResponse else { return }
        if http.statusCode != expectedStatus {
            throw PromptKeeperError.httpStatus(http.statusCode, body: data)
        }
    }
}

// MARK: - Auth

private extension URLRequest {
    mutating func setAuth(_ apiKey: String) {
        setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        setValue(apiKey, forHTTPHeaderField: "X-API-Key")
    }
}
