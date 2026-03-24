//
//  ExecService.swift
//  PromptKeeperExample
//
//  Management key: setPrompt. Execution key: listPrompts + exec (streaming).
//

import Foundation
import PromptKeeper

@MainActor
final class ExecService: ObservableObject {
    private var management: PromptKeeper?
    private var execution: PromptKeeper?

    init() {
        reloadClientsFromKeys()
    }

    func reloadClientsFromKeys() {
        management = Keys.managementAPIKey.isEmpty ? nil : PromptKeeper(apiKey: Keys.managementAPIKey)
        execution = Keys.executionAPIKey.isEmpty ? nil : PromptKeeper(apiKey: Keys.executionAPIKey)
    }

    var hasManagementKey: Bool { management != nil }
    var hasExecutionKey: Bool { execution != nil }

    /// Stores a named prompt template (`POST /v1/prompts`) using the **management** API key.
    func storePrompt(title: String, promptText: String, provider: String?) async throws -> PutPromptResponse {
        guard let client = management else { throw ExecError.managementKeyNotConfigured }
        let name = title.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = promptText.trimmingCharacters(in: .whitespacesAndNewlines)
        let prov = provider?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let providerArg: String? = (prov?.isEmpty == false) ? prov : nil
        return try await client.setPrompt(
            name: name,
            rawSecret: body,
            provider: providerArg,
            preferredModel: nil
        )
    }

    /// Lists stored prompt titles (`GET /v1/list-prompts`) using the **execution** API key.
    func listPromptTitles() async throws -> [String] {
        guard let client = execution else { throw ExecError.executionKeyNotConfigured }
        return try await client.listPrompts(surface: "ios")
    }

    /// Runs a stored prompt by function id (`POST /v1/execute`) using the **execution** API key.
    func runStreamingExec(
        functionId: String,
        provider: String? = nil,
        model: String? = nil,
        onChunk: @escaping (String) -> Void
    ) async throws {
        guard let client = execution else { throw ExecError.executionKeyNotConfigured }
        let stream = client.exec(
            functionId: functionId,
            variables: [:],
            provider: provider,
            model: model,
            surface: "ios"
        )
        for try await event in stream {
            if case .chunk(let data) = event {
                let parsed = Self.parseStreamChunk(data)
                onChunk(parsed ?? data)
            }
        }
    }

    private static func parseStreamChunk(_ chunk: String) -> String? {
        if let o = OpenAIStreamParser.parseDeltaContent(from: chunk) { return o }
        if let g = GeminiStreamParser.parseText(from: chunk) { return g }
        if let a = AnthropicStreamParser.parseText(from: chunk) { return a }
        return nil
    }
}

enum ExecError: LocalizedError {
    case managementKeyNotConfigured
    case executionKeyNotConfigured

    var errorDescription: String? {
        switch self {
        case .managementKeyNotConfigured:
            return "Management API key is not set. Add `pk_mgt_live_…` to Keys.managementAPIKey."
        case .executionKeyNotConfigured:
            return "Execution API key is not set. Add `pk_exe_live_…` to Keys.executionAPIKey."
        }
    }
}
