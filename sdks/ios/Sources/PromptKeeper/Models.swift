//
//  Models.swift
//  PromptKeeper
//
//  Request/response DTOs for Prompt Keeper API.
//

import Foundation

// MARK: - Set key

struct PutKeyRequest: Encodable {
    let raw_secret: String
    let provider: String
}

/// Response from setKey (store provider API key).
public struct PutKeyResponse: Decodable, Sendable {
    public let version_id: String
    public let created_at: String
    public let kms_key_arn: String?
    public let fingerprint: String?
}

// MARK: - Set prompt

struct PutPromptRequest: Encodable {
    let name: String
    let raw_secret: String
    let provider: String?
    let preferred_model: String?
}

/// Response from setPrompt (store prompt template).
public struct PutPromptResponse: Decodable, Sendable {
    public let version_id: String
    public let created_at: String
    public let kms_key_arn: String?
    public let fingerprint: String?
}

// MARK: - Execute

struct ExecuteRequest: Encodable {
    let function_id: String
    let variables: [String: String]
    let provider: String?
    let model: String?
}

/// One item from the exec SSE stream.
public enum ExecStreamEvent: Sendable {
    /// A data chunk from the LLM stream (provider-specific payload).
    case chunk(String)
}

// MARK: - Errors

public enum PromptKeeperError: Error, Sendable {
    case httpStatus(Int, body: Data)
    case serverError(String)

    public var message: String {
        switch self {
        case .httpStatus(let code, let body):
            if let str = String(data: body, encoding: .utf8), !str.isEmpty {
                return "HTTP \(code): \(str)"
            }
            return "HTTP \(code)"
        case .serverError(let msg):
            return msg
        }
    }
}
