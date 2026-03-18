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

    /// Runs a text-generation exec by stored function id (e.g. OpenAI), streaming parsed content.
    func runTextExec(
        functionId: String,
        variables: [String: String],
        provider: String? = "openai",
        model: String? = nil,
        onChunk: @escaping (String) -> Void
    ) async throws {
        guard let client = client else { throw ExecError.notConfigured }
        let stream = client.exec(functionId: functionId, variables: variables, provider: provider, model: model)
        for try await event in stream {
            if case .chunk(let data) = event {
                if let text = OpenAIStreamParser.parseDeltaContent(from: data) {
                    onChunk(text)
                }
            }
        }
    }

    /// Runs a text-generation exec using an inline prompt (no stored function). Uses `execPrompt`.
    func runTextExecPrompt(
        prompt: String,
        variables: [String: String] = [:],
        provider: String? = "openai",
        model: String? = nil,
        onChunk: @escaping (String) -> Void
    ) async throws {
        guard let client = client else { throw ExecError.notConfigured }
        let providerNormalized = (provider ?? "openai").trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let stream = client.execPrompt(
            prompt: prompt,
            variables: variables.isEmpty ? nil : variables,
            provider: providerNormalized,
            model: model
        )
        for try await event in stream {
            if case .chunk(let data) = event {
                // Provider-specific extraction with a safe fallback to raw chunk text.
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
