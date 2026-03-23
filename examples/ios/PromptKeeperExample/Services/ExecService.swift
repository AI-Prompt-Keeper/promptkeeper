//
//  ExecService.swift
//  PromptKeeperExample
//
//  Uses PromptKeeper SDK for exec calls (no manual networking). Parses stream chunks per provider.
//

import Foundation
import PromptKeeper

@MainActor
final class ExecService: ObservableObject {
    private var client: PromptKeeper?

    var isConfigured: Bool { client != nil }

    func configure(apiKey: String) {
        client = PromptKeeper(apiKey: apiKey)
    }

    /// Registers a minimal stored prompt `text` with `{{prompt}}` so the Text tab can run via POST /v1/execute.
    func ensureTextPromptTemplate() async throws {
        guard let client = client else { throw ExecError.notConfigured }
        _ = try await client.setPrompt(
            name: "text",
            rawSecret: "{{prompt}}",
            provider: nil,
            preferredModel: nil
        )
    }

    /// Runs a text-generation exec by stored function id, streaming parsed content per provider.
    func runTextExec(
        functionId: String,
        variables: [String: String],
        provider: String? = "openai",
        model: String? = nil,
        onChunk: @escaping (String) -> Void
    ) async throws {
        guard let client = client else { throw ExecError.notConfigured }
        let providerNormalized = (provider ?? "openai").trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let stream = client.exec(functionId: functionId, variables: variables, provider: providerNormalized, model: model)
        for try await event in stream {
            if case .chunk(let data) = event {
                let parsed: String?
                switch providerNormalized {
                case "openai":
                    parsed = OpenAIStreamParser.parseDeltaContent(from: data)
                case "gemini":
                    parsed = GeminiStreamParser.parseText(from: data)
                case "anthropic":
                    parsed = AnthropicStreamParser.parseText(from: data)
                default:
                    parsed = nil
                }
                onChunk(parsed ?? data)
            }
        }
    }

    /// Runs an image-generation exec (e.g. Gemini), returning the first decoded image data or nil.
    func runImageExec(
        functionId: String,
        variables: [String: String],
        provider: String? = "gemini",
        model: String? = nil
    ) async throws -> Data? {
        guard let client = client else { throw ExecError.notConfigured }
        var result: Data?
        let stream = client.exec(functionId: functionId, variables: variables, provider: provider, model: model)
        for try await event in stream {
            if case .chunk(let data) = event {
                if let imageData = GeminiStreamParser.parseInlineImageBase64(from: data) {
                    result = imageData
                    break
                }
                if GeminiStreamParser.parseText(from: data) != nil {
                    // Optional: accumulate or skip text in image flow
                }
            }
        }
        return result
    }
}

enum ExecError: LocalizedError {
    case notConfigured
    var errorDescription: String? {
        switch self {
        case .notConfigured: return "Prompt Keeper API key not set. Configure in the app first."
        }
    }
}
